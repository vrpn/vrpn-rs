// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{buf::IntoBuf, Buf, BufMut, Bytes, BytesMut};
use crate::prelude::*;
use crate::{
    async_io::{
        connection_ip::ConnectionIpAcceptor,
        cookie::{read_and_check_nonfile_cookie, send_nonfile_cookie},
        ping, ConnectionIp, ConnectionIpStream, Drain, StreamExtras,
    },
    constants,
    cookie::check_ver_nonfile_compatible,
    ConnectionStatus, CookieData, Error, Result, Scheme, ServerInfo, Unbuffer,
};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{
    fmt::{self, Debug},
    net::{self, IpAddr, SocketAddr, ToSocketAddrs},
    time::{Duration, Instant},
};
use tk_listen::{ListenExt, SleepOnError};
use tokio::prelude::*;
use tokio::{
    io,
    net::{
        tcp::{ConnectFuture, Incoming, TcpListener},
        TcpStream, UdpSocket,
    },
    timer::Delay,
};

pub fn make_tcp_socket(addr: SocketAddr) -> io::Result<net::TcpStream> {
    use socket2::*;
    let domain = if addr.is_ipv4() {
        Domain::ipv4()
    } else {
        Domain::ipv6()
    };
    let sock = socket2::Socket::new(domain, Type::stream(), Some(Protocol::tcp()))?;
    sock.set_nonblocking(true)?;
    sock.set_nodelay(true)?;

    if cfg!(windows) {
        if addr.is_ipv4() {
            let any = std::net::Ipv4Addr::new(0, 0, 0, 0);
            let addr = std::net::SocketAddrV4::new(any, 0);
            sock.bind(&socket2::SockAddr::from(addr))?;
        } else {
            unimplemented!();
        }
    }
    sock.set_reuse_address(true)?;
    Ok(sock.into_tcp_stream())
}

pub fn make_udp_socket() -> io::Result<UdpSocket> {
    let domain = Domain::ipv4();
    let sock = Socket::new(domain, Type::dgram(), Some(Protocol::udp()))?;
    sock.set_nonblocking(true)?;
    sock.set_nodelay(true)?;

    let any = std::net::Ipv4Addr::new(0, 0, 0, 0);
    let addr = SocketAddr::new(IpAddr::V4(any), 0);
    sock.bind(&SockAddr::from(addr))?;
    sock.set_reuse_address(true)?;
    let tokio_socket =
        UdpSocket::from_std(sock.into_udp_socket(), &tokio::reactor::Handle::default())?;
    Ok(tokio_socket)
}

pub fn outgoing_tcp_connect(addr: std::net::SocketAddr) -> Result<ConnectFuture> {
    let sock = make_tcp_socket(addr).map_err(|e| Error::from(e))?;

    Ok(TcpStream::connect_std(
        sock,
        &addr,
        &tokio::reactor::Handle::default(),
    ))
}

pub fn outgoing_handshake<T>(socket: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncRead + AsyncWrite,
{
    send_nonfile_cookie(socket).and_then(read_and_check_nonfile_cookie)
    // TODO can pack log description here if we're enabling remote logging.
    // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
}

// pub fn connect_tcp(
//     addr: std::net::SocketAddr,
// ) -> impl Future<Item = tokio::net::TcpStream, Error = Error> {
//     outgoing_tcp_connect(addr)
//         .into_future()
//         .and_then(outgoing_handshake)
//     // TODO can pack log description here if we're enabling remote logging.
//     // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
// }

pub fn incoming_handshake<T>(socket: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncRead + AsyncWrite,
{
    // If connection is incoming
    read_and_check_nonfile_cookie(socket).and_then(send_nonfile_cookie)

    // TODO can pack log description here if we're enabling remote logging.
    // TODO should send descriptions here.
}

/// A separate future, because couldn't get a boxed future built with combinators
/// to appease the borrow checker for threading reasons.
struct WaitForConnect {
    ip: IpAddr,
    incoming: SleepOnError<Incoming>,
}
impl Debug for WaitForConnect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "waiting for connection from {}", self.ip)
    }
}
impl WaitForConnect {
    fn new(ip: IpAddr, listener: TcpListener) -> WaitForConnect {
        WaitForConnect {
            ip,
            incoming: listener
                .incoming()
                .sleep_on_error(Duration::from_millis(100)),
        }
    }
}

impl Future for WaitForConnect {
    type Item = TcpStream;
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let result = try_ready!(self.incoming.poll());
            if let Some(stream) = result {
                if stream.peer_addr().map_err(|_| ())?.ip() == self.ip {
                    return Ok(Async::Ready(stream));
                }
            }
        }
    }
}

/// The steps of establishing a connection
#[derive(Debug)]
enum State {
    /// Sending the initial UDP datagram with our "call-back" address and port
    Lobbing(Option<TcpListener>, IpAddr),
    /// Follows after Lobbing.
    WaitingForConnection(WaitForConnect),
    /// Making the connection for a TCP-only setup.
    Connecting(ConnectFuture),
    /// Reached from Connecting in case of error.
    DelayBeforeConnectionRetry,
    /// Transmitting the magic cookie - used by both modes.
    SendingHandshake,
    /// Receiving and checking the magic cookie - used by both modes.
    ReceivingHandshake(BytesMut),
}

pub(crate) struct ConnectResults {
    pub(crate) tcp: Option<TcpStream>,
    pub(crate) udp: Option<UdpSocket>,
}

/// Connect members that only are populated for UDP connections.
#[derive(Debug)]
struct UdpConnect {
    udp: UdpSocket,
    lobbed_buf: Bytes,
}

enum ConnectPollOutput {
    Connected,
    NotConnected,
}

/// A future that handles the connection and handshake process.
#[derive(Debug)]
pub(crate) struct Connect {
    state: Option<State>,
    server: ServerInfo,
    stream: Option<TcpStream>,
    udp_connect: Option<UdpConnect>,
    cookie_buf: <Bytes as IntoBuf>::Buf,
    delay: Option<Delay>,
}
const MILLIS_BETWEEN_ATTEMPTS: u64 = 500;

fn set_delay(delay: &mut Option<Delay>) {
    let deadline = Instant::now() + Duration::from_millis(MILLIS_BETWEEN_ATTEMPTS);
    match delay.as_mut() {
        Some(d) => {
            d.reset(deadline);
        }
        None => {
            *delay = Some(Delay::new(deadline));
        }
    };
}
impl Connect {
    /// Create a future for establishing a connection.
    pub(crate) fn new(server: ServerInfo) -> Result<Connect> {
        let cookie_buf = BytesMut::new()
            .allocate_and_buffer(CookieData::from(constants::MAGIC_DATA))?
            .freeze()
            .into_buf();
        match server.scheme {
            Scheme::UdpAndTcp => {
                let udp = make_udp_socket()?;
                let addr = "localhost".to_socket_addrs()?.next().unwrap();
                let addr = SocketAddr::new(addr.ip(), 0);
                let tcp_listener = TcpListener::bind(&addr)?;
                let port = udp.local_addr()?.port();
                let addr = SocketAddr::new(addr.ip(), port);
                let lobbed_buf = {
                    let addr_str = addr.ip().to_string();
                    let port_str = addr.port().to_string();
                    let mut buf = BytesMut::with_capacity(addr_str.len() + port_str.len() + 2);

                    buf.put(addr_str);
                    buf.put(" ");
                    buf.put(port_str);
                    buf.put_u8(0);
                    buf
                };
                let ip = addr.ip();
                let lobbed_buf = lobbed_buf.freeze();
                Ok(Connect {
                    server,
                    udp_connect: Some(UdpConnect { udp, lobbed_buf }),
                    state: Some(State::Lobbing(Some(tcp_listener), ip)),
                    cookie_buf,
                    stream: None,
                    delay: None,
                })
            }
            Scheme::TcpOnly => {
                let addr = server.socket_addr.clone();
                let connect_future = outgoing_tcp_connect(addr)?;
                Ok(Connect {
                    server,
                    udp_connect: None,
                    state: Some(State::Connecting(connect_future)),
                    cookie_buf,
                    stream: None,
                    delay: None,
                })
            }
        }
    }

    fn poll_one(&mut self) -> Poll<ConnectPollOutput, Error> {
        let state = self.state.as_mut().unwrap();
        match state {
            State::Lobbing(tcp_listener, ip) => {
                if let Some(udp_connect) = self.udp_connect.as_mut() {
                    try_ready!(udp_connect
                        .udp
                        .poll_send_to(&udp_connect.lobbed_buf, &self.server.socket_addr));
                    //if we don't return immediately, then we're OK.
                    *state = State::WaitingForConnection(WaitForConnect::new(
                        *ip,
                        tcp_listener.take().unwrap(),
                    ));
                    return Ok(Async::Ready(ConnectPollOutput::NotConnected));
                } else {
                    return Err(Error::OtherMessage(String::from("no udp socket found?")));
                }
            }
            State::Connecting(conn_future) => match conn_future.poll() {
                Err(e) => {
                    eprintln!("Error connecting: {}. Will retry after a delay.", e);
                    set_delay(&mut self.delay);
                    *state = State::DelayBeforeConnectionRetry;
                    return Ok(Async::Ready(ConnectPollOutput::NotConnected));
                }
                Ok(Async::Ready(stream)) => {
                    self.stream = Some(stream);
                    *state = State::SendingHandshake;
                }
                Ok(Async::NotReady) => {
                    return Ok(Async::NotReady);
                }
            },

            State::DelayBeforeConnectionRetry => {
                let _ = try_ready!(self
                    .delay
                    .as_mut()
                    .unwrap()
                    .poll()
                    .map_err(|e| Error::OtherMessage(e.to_string())));
                if let Ok(connect_future) = outgoing_tcp_connect(self.server.socket_addr.clone()) {
                    eprintln!("Delay completed, and we were able to connect.");
                    *state = State::Connecting(connect_future);
                } else {
                    eprintln!("Delay completed but still could not connect to server.");
                    set_delay(&mut self.delay);
                }
            }
            State::WaitingForConnection(conn_stream) => {
                let stream = try_ready!(conn_stream
                    .poll()
                    .map_err(|_e| Error::OtherMessage(String::from(""))));
                self.stream = Some(stream);
                *state = State::SendingHandshake;
            }
            State::SendingHandshake => {
                while self.cookie_buf.has_remaining() {
                    try_ready!(self
                        .stream
                        .as_mut()
                        .unwrap()
                        .write_buf(&mut self.cookie_buf));
                }
                let cookie_size = CookieData::constant_buffer_size();
                let mut buf = BytesMut::with_capacity(cookie_size);
                *state = State::ReceivingHandshake(buf);
            }
            State::ReceivingHandshake(buf) => {
                while buf.len() < CookieData::constant_buffer_size() {
                    let _ = try_ready!(self.stream.as_mut().unwrap().read_buf(buf));
                }
                let mut buf = buf.clone().freeze();
                let cookie = CookieData::unbuffer_ref(&mut buf)?;
                check_ver_nonfile_compatible(cookie.version)?;
                return Ok(Async::Ready(ConnectPollOutput::Connected));
            }
        };

        return Ok(Async::Ready(ConnectPollOutput::NotConnected));
    }
}

impl Future for Connect {
    type Item = ConnectResults;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.poll_one()? {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(ConnectPollOutput::NotConnected) => {}
                Async::Ready(ConnectPollOutput::Connected) => {
                    let udp = self.udp_connect.take().map(|udp_connect| udp_connect.udp);

                    return Ok(Async::Ready(ConnectResults {
                        tcp: self.stream.take(),
                        udp,
                    }));
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ConnectionIpInfo {
    ConnectionSetupFuture(Connect),
    Info(ServerInfo),
    Server,
}

impl ConnectionIpInfo {
    pub(crate) fn new_client(server: ServerInfo) -> Result<ConnectionIpInfo> {
        Ok(ConnectionIpInfo::ConnectionSetupFuture(Connect::new(
            server,
        )?))
    }

    pub(crate) fn new_server() -> Result<ConnectionIpInfo> {
        Ok(ConnectionIpInfo::Server)
    }
    pub(crate) fn poll(&mut self, num_endpoints: usize) -> Poll<Option<ConnectResults>, Error> {
        loop {
            match self {
                ConnectionIpInfo::ConnectionSetupFuture(fut) => {
                    let result = try_ready!(fut.poll());
                    *self = ConnectionIpInfo::Info(fut.server.clone());
                    return Ok(Async::Ready(Some(result)));
                }
                ConnectionIpInfo::Info(info) => {
                    if num_endpoints == 0 {
                        eprintln!("No endpoints, despite claims we've already connected. Re-starting connection process.");
                        *self = ConnectionIpInfo::new_client(info.clone())?;
                    } else {
                        return Ok(Async::Ready(None));
                    }
                }
                _ => return Ok(Async::Ready(None)),
            }
        }
    }
    pub(crate) fn status(&self, num_endpoints: usize) -> ConnectionStatus {
        match *self {
            ConnectionIpInfo::ConnectionSetupFuture(_) => ConnectionStatus::ClientConnecting,
            ConnectionIpInfo::Info(_) => ConnectionStatus::ClientConnected,
            ConnectionIpInfo::Server => ConnectionStatus::Server(num_endpoints),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use crate::{
        constants::MAGIC_DATA, cookie::check_ver_nonfile_compatible, ConstantBufferSize,
        CookieData, ServerInfo, Unbuffer,
    };

    #[test]
    fn basic_connect_tcp() {
        let results = "tcp://127.0.0.1:3883"
            .parse::<ServerInfo>()
            .into_future()
            .and_then(|server| Connect::new(server))
            .flatten()
            .wait()
            .expect("should be able to create connection future");
        results.tcp.expect("Should have a TCP stream");
        assert!(results.udp.is_none());
    }
    #[test]
    fn basic_connect() {
        let results = "127.0.0.1:3883"
            .parse::<ServerInfo>()
            .into_future()
            .and_then(|server| Connect::new(server))
            .flatten()
            .wait()
            .expect("should be able to create connection future");
        results.tcp.expect("Should have a TCP stream");
        assert!(results.udp.is_some());
    }

    #[test]
    fn sync_connect() {
        use crate::buffer::Buffer;

        let addr = "127.0.0.1:3883".parse().unwrap();

        let sock = make_tcp_socket(addr).expect("failure making the socket");
        let stream = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default())
            .wait()
            .unwrap();

        let cookie = CookieData::from(MAGIC_DATA);
        let mut send_buf = BytesMut::with_capacity(cookie.required_buffer_size());
        cookie.buffer_ref(&mut send_buf).unwrap();
        let (stream, _) = io::write_all(stream, send_buf.freeze()).wait().unwrap();

        let (_stream, read_buf) =
            io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()])
                .wait()
                .unwrap();
        let mut read_buf = Bytes::from(read_buf);
        let parsed_cookie: CookieData = Unbuffer::unbuffer_ref(&mut read_buf).unwrap();
        check_ver_nonfile_compatible(parsed_cookie.version).unwrap();
    }
}
