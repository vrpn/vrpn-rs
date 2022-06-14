// Copyright 2018-2022, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::cookie::{read_and_check_nonfile_cookie, send_nonfile_cookie};
use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::{cookie::check_ver_nonfile_compatible, CookieData},
    Result, Scheme, ServerInfo, VrpnError, ConnectionStatus,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::ready;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::future::Future;
use std::task::Poll;
use std::{
    fmt::{self, Debug},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
};
use tokio::io::AsyncWriteExt;
use tokio::{
    io,
    net::{TcpListener, UdpSocket},
};

pub fn make_tcp_socket(addr: SocketAddr) -> io::Result<socket2::Socket> {
    use socket2::*;
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let sock = socket2::Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
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
    Ok(sock)
}

pub fn make_udp_socket() -> io::Result<UdpSocket> {
    let domain = Domain::IPV4;
    let sock = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    sock.set_nonblocking(true)?;
    sock.set_nodelay(true)?;

    let any = std::net::Ipv4Addr::new(0, 0, 0, 0);
    let addr = SocketAddr::new(IpAddr::V4(any), 0);
    sock.bind(&SockAddr::from(addr))?;
    sock.set_reuse_address(true)?;
    let tokio_socket = UdpSocket::from_std(std::net::UdpSocket::from(sock))?;
    Ok(tokio_socket)
}

pub async fn outgoing_tcp_connect(addr: std::net::SocketAddr) -> Result<tokio::net::TcpStream> {
    let sock = make_tcp_socket(addr)?;
    sock.connect(&SockAddr::from(addr))?;
    Ok(tokio::net::TcpStream::from_std(std::net::TcpStream::from(
        sock,
    ))?)
}

pub async fn outgoing_handshake<T>(socket: &mut T) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    send_nonfile_cookie(socket).await?;
    read_and_check_nonfile_cookie(socket).await
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

pub async fn incoming_handshake<T>(socket: &mut T) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // If connection is incoming
    read_and_check_nonfile_cookie(socket).await?;
    send_nonfile_cookie(socket).await

    // TODO can pack log description here if we're enabling remote logging.
    // TODO should send descriptions here.
}

/// A separate future, because couldn't get a boxed future built with combinators
/// to appease the borrow checker for threading reasons.
pub(crate) struct WaitForConnect {
    ip: IpAddr,
    listener: TcpListener,
}

impl Debug for WaitForConnect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "waiting for connection from {}", self.ip)
    }
}
impl WaitForConnect {
    fn new(ip: IpAddr, listener: TcpListener) -> WaitForConnect {
        WaitForConnect { ip, listener }
    }
}

impl Future for WaitForConnect {
    // fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    //     loop {
    //         let result = ready!(self.incoming.poll());
    //         if let Some(stream) = result {
    //             if stream.peer_addr().map_err(|_| ())?.ip() == self.ip {
    //                 return Poll::Ready(stream);
    //             }
    //         }
    //     }
    // }

    type Output = tokio::net::TcpStream;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let result = ready!(self.listener.poll_accept(cx));
            if let Ok((stream, _)) = result {
                if let Ok(peer) = stream.peer_addr() {
                    if peer.ip() == self.ip {
                        return Poll::Ready(stream);
                    }
                }
            }
        }
    }
}

// async fn wait_for_connect(ip: IpAddr, listener: TcpListener) -> TcpStream {}

/// The steps of establishing a connection
// #[derive(Debug)]
pub(crate) enum State {
    /// Sending the initial UDP datagram with our "call-back" address and port
    Lobbing(Option<TcpListener>, IpAddr),
    /// Follows after Lobbing.
    WaitingForConnection(WaitForConnect),
    /// Making the connection for a TCP-only setup.
    Connecting,
    /// Reached from Connecting in case of error.
    DelayBeforeConnectionRetry,
    /// Transmitting the magic cookie - used by both modes.
    SendingHandshake,
    /// Receiving and checking the magic cookie - used by both modes.
    ReceivingHandshake(BytesMut),
}

impl Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lobbing(arg0, arg1) => f.debug_tuple("Lobbing").field(arg0).field(arg1).finish(),
            Self::WaitingForConnection(_) => write!(f, "WaitingForConnection"),
            Self::Connecting => write!(f, "Connecting"),
            Self::DelayBeforeConnectionRetry => write!(f, "DelayBeforeConnectionRetry"),
            Self::SendingHandshake => write!(f, "SendingHandshake"),
            Self::ReceivingHandshake(arg0) => {
                f.debug_tuple("ReceivingHandshake").field(arg0).finish()
            }
        }
    }
}

pub struct ConnectResults {
    pub(crate) tcp: Option<tokio::net::TcpStream>,
    pub(crate) udp: Option<UdpSocket>,
}

/// Connect members that only are populated for UDP connections.
#[derive(Debug)]
pub(crate) struct UdpConnect {
    udp: UdpSocket,
    lobbed_buf: Bytes,
}

// impl Debug for Option<Box<dyn Future<Output = TcpStream>>> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Self::None => write!(f, "None"),
//             Self::Some(arg0) => f.debug("Some Future").finish(),
//         }
//     }
// }

/// A future that handles the connection and handshake process.
#[derive(Debug)]
pub(crate) struct Connect {
    state: Option<State>,
    server: ServerInfo,
    // stream: Option<tokio::net::TcpStream>,
    udp_connect: Option<UdpConnect>,
    // cookie_buf: Bytes,
    // delay: Box<Sleep>,
    // current_future: Option<Box<dyn Future<Output = tokio::net::TcpStream> + Unpin>>,
}
const MILLIS_BETWEEN_ATTEMPTS: u64 = 500;

// fn set_delay(delay: &mut Option<Delay>) {
//     let deadline = Instant::now() + Duration::from_millis(MILLIS_BETWEEN_ATTEMPTS);
//     match delay.as_mut() {
//         Some(d) => {
//             d.reset(deadline);
//         }
//         None => {
//             *delay = Some(Delay::new(deadline));
//         }
//     };
// }

// fn set_delay(delay: &mut Box<Sleep>) {
//     let deadline = Instant::now() + Duration::from_millis(MILLIS_BETWEEN_ATTEMPTS);
//     delay. = tokio::time::sleep(tokio::time::Instant::from_std(deadline)).boxed();
//     delay.reset();
// }

// impl Connect {
//     fn delay_before_retry(self: std::pin::Pin<&mut Self>) {

//         set_delay(&mut self.delay);
//         self.state = Some(State::DelayBeforeConnectionRetry);
//     }
// }

// impl Future for Connect {
//     type Output = Result<ConnectResults>;

//     fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
//         // Loop until we succeed, error, or hit NotReady
//         let stream: Option<tokio::net::TcpStream> = None;
//         loop {
//             // Handle each different state.
//             let state = self.state.as_mut().unwrap();
//             match state {
//                 State::Lobbing(tcp_listener, ip) => {
//                     if let Some(udp_connect) = self.udp_connect.as_mut() {
//                         ready!(udp_connect
//                             .udp
//                             .poll_send_to(cx, &udp_connect.lobbed_buf, self.server.socket_addr.clone()));
//                         //if we don't return immediately, then we're OK.
//                         *state = State::WaitingForConnection( WaitForConnect::new(
//                             *ip,
//                             tcp_listener.take().unwrap(),
//                         ));
//                     } else {
//                         return Poll::Ready(Err(Error::OtherMessage(String::from("no udp socket found?"))));
//                     }
//                 }

//                 State::Connecting(conn_future) => match ready!(conn_future.poll_unpin(cx)) {

//                     Err(e) => {
//                         eprintln!("Error connecting: {}. Will retry after a delay.", e);
//                         self.delay_before_retry();
//                         return Poll::Pending;

//                     }
//                     Ok(stream)  => {
//                         self.stream = Some(stream);
//                         *state = State::SendingHandshake;
//                         return Poll::Pending;
//                     }
//                 },

//                 State::DelayBeforeConnectionRetry => {
//                     connect.delay.await;
//                     eprintln!("Delay completed.");
//                     *state = State::Connecting(Box::new(outgoing_tcp_connect(self.server.socket_addr)));
//                     // match connect_future.poll_unpin(cx) {
//                     //     Poll::Pending => {
//                     //         *state = State::Connecting(Box::new(connect_future));
//                     //         return Poll::Pending;

//                     //     }
//                     //     Poll::Ready(Err(e))=> {
//                     //         eprintln!("Delay completed but still could not connect to server: {}", e);
//                     //         self.delay_before_retry();
//                     //         return Poll::Pending;
//                     //     }
//                     //     Poll::Ready(Ok(connect_future)) => {
//                     //         eprintln!("Delay completed, and we were able to connect.");
//                     //     }
//                     //     Err(e) => {
//                     //                         eprintln!("Delay completed but still could not connect to server: {}", e);
//                     //                         set_delay(&mut self.delay);
//                     //                         return Poll::Pending;
//                     //                     }
//                     // }
//                 }

//                 State::WaitingForConnection(conn_stream) => {
//                     let stream = ready!(conn_stream
//                         .poll_unpin(cx));
//                     self.stream = Some(stream);
//                     *state = State::SendingHandshake;
//                 }

//                 State::SendingHandshake => {
//                     if let  Some(stream) =  self.stream.as_mut() {
//                         AsyncWr stream.(&mut self.cookie_buf)
//                     }
//                     while self.cookie_buf.has_remaining() {
//                         ready!(self
//                             .stream
//                             .as_mut()
//                             .unwrap()
//                             .write_buf(&mut self.cookie_buf).);
//                     }
//                     let cookie_size = CookieData::constant_buffer_size();
//                     let buf = BytesMut::with_capacity(cookie_size);
//                     *state = State::ReceivingHandshake(buf);
//                 }

//                 State::ReceivingHandshake(buf) => {
//                     while buf.len() < CookieData::constant_buffer_size() {
//                         let _ = ready!(self.stream.as_mut().unwrap().try_read(buf));
//                     }
//                     let mut buf = buf.clone().freeze();
//                     let cookie = CookieData::unbuffer_ref(&mut buf)?;
//                     check_ver_nonfile_compatible(cookie.version)?;
//                     let udp = self.udp_connect.take().map(|udp_connect| udp_connect.udp);

//                     return Poll::Ready(Ok(ConnectResults {
//                         tcp: self.stream.take(),
//                         udp,
//                     }));
//                 }
//             };
//         }
//     }
// }

async fn connect_tcp_and_udp(server: ServerInfo) -> Result<ConnectResults> {
    let udp = make_udp_socket()?;
    let addr = "localhost".to_socket_addrs()?.next().unwrap();
    let addr = SocketAddr::new(addr.ip(), 0);
    let tcp_listener = TcpListener::bind(&addr).await?;
    let port = udp.local_addr()?.port();
    let addr = SocketAddr::new(addr.ip(), port);
    let lobbed_buf = {
        let addr_str = addr.ip().to_string();
        let port_str = addr.port().to_string();
        let mut buf = BytesMut::with_capacity(addr_str.len() + port_str.len() + 2);

        buf.put(addr_str.as_bytes());
        buf.put(" ".as_bytes());
        buf.put(port_str.as_bytes());
        buf.put_u8(0);
        buf
    };
    let ip = addr.ip();
    let lobbed_buf = lobbed_buf.freeze();
    finish_connecting(
        server,
        State::Lobbing(Some(tcp_listener), ip),
        Some(UdpConnect { udp, lobbed_buf }),
    )
    .await
}
async fn connect_tcp_only(server: ServerInfo) -> Result<ConnectResults> {
    let cookie_buf = BytesMut::allocate_and_buffer(CookieData::make_cookie())?.freeze();
    let addr = server.socket_addr;
    finish_connecting(server, State::Connecting, None).await
}

pub(crate) async fn finish_connecting(
    server: ServerInfo,
    state: State,
    udp_connect: Option<UdpConnect>,
) -> Result<ConnectResults> {
    let mut full_state = Some(state);
    let mut udp_connect = udp_connect;

    let mut stream: Option<tokio::net::TcpStream> = None;
    async fn delay_before_retry() {
        tokio::time::sleep(tokio::time::Duration::from_millis(MILLIS_BETWEEN_ATTEMPTS)).await
    }
    // Loop until we succeed, error, or hit NotReady
    loop {
        let state = full_state.as_mut().unwrap();
        // Handle each different state.
        match state {
            State::Lobbing(tcp_listener, ip) => {
                if let Some(udp_connect) = udp_connect.as_mut() {
                    udp_connect
                        .udp
                        .send_to(&udp_connect.lobbed_buf, server.socket_addr.clone())
                        .await?;
                    *state = State::WaitingForConnection(WaitForConnect::new(
                        *ip,
                        tcp_listener.take().unwrap(),
                    ));
                } else {
                    return Err(VrpnError::OtherMessage(String::from(
                        "no udp socket found?",
                    )));
                }
            }

            State::Connecting => match outgoing_tcp_connect(server.socket_addr).await {
                Err(e) => {
                    eprintln!("Error connecting: {}. Will retry after a delay.", e);
                    *state = State::DelayBeforeConnectionRetry;
                }
                Ok(s) => {
                    stream = Some(s);
                    *state = State::SendingHandshake;
                }
            },

            State::DelayBeforeConnectionRetry => {
                delay_before_retry().await;
                eprintln!("Delay completed.");
                *state = State::Connecting;
            }

            State::WaitingForConnection(conn_stream) => {
                stream = Some(conn_stream.await);
                *state = State::SendingHandshake;
            }

            State::SendingHandshake => {
                let mut cookie_buf =
                    BytesMut::allocate_and_buffer(CookieData::make_cookie())?.freeze();
                while cookie_buf.has_remaining() {
                    stream.as_mut().unwrap().write_buf(&mut cookie_buf).await?;
                }
                let cookie_size = CookieData::constant_buffer_size();
                let buf = BytesMut::with_capacity(cookie_size);
                *state = State::ReceivingHandshake(buf);
            }

            State::ReceivingHandshake(buf) => {
                while buf.len() < CookieData::constant_buffer_size() {
                    let _ = stream.as_mut().unwrap().try_read(buf);
                }
                let mut buf = buf.clone().freeze();
                let cookie = CookieData::unbuffer_from(&mut buf)?;
                check_ver_nonfile_compatible(cookie.version)?;
                let udp = udp_connect.take().map(|udp_connect| udp_connect.udp);

                return Ok(ConnectResults {
                    tcp: stream.take(),
                    udp,
                });
            }
        };
    }
}

pub async fn connect(server: ServerInfo) -> Result<ConnectResults> {
    match server.scheme {
        Scheme::UdpAndTcp => connect_tcp_and_udp(server).await,
        Scheme::TcpOnly => connect_tcp_only(server).await,
    }
}
impl Connect {
    pub async fn new(server: ServerInfo) -> Result<Self> {
        match server.scheme {
            Scheme::UdpAndTcp => connect_tcp_and_udp(server).await,
            Scheme::TcpOnly => connect_tcp_only(server).await,
        }
    }
}
// pub(crate) async fn connect(server: ServerInfo) -> Result<()> {
//     let mut connect: Option<Connect> = None;
//     match server.scheme {
//         Scheme::UdpAndTcp => {}
//         Scheme::TcpOnly => {}
//     }
// }
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
    pub(crate) fn poll(&mut self, num_endpoints: usize) -> Poll<Result<Option<ConnectResults>>> {
        loop {
            match self {
                ConnectionIpInfo::ConnectionSetupFuture(fut) => {
                    let result = ready!(fut.poll());
                    *self = ConnectionIpInfo::Info(fut.server.clone());
                    return Ok(Poll::Ready(Some(result)));
                }
                ConnectionIpInfo::Info(info) => {
                    if num_endpoints == 0 {
                        eprintln!("No endpoints, despite claims we've already connected. Re-starting connection process.");
                        *self = ConnectionIpInfo::new_client(info.clone())?;
                    } else {
                        return Ok(Poll::Ready(None));
                    }
                }
                _ => return Ok(Poll::Ready(None)),
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
    use std::io::{Read, Write};

    use super::*;
    use bytes::{Bytes, BytesMut};

    #[test]
    fn basic_connect_tcp() {
        let results = tokio_test::block_on(connect(
            "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap(),
        ))
        .expect("should be able to connect");
        results.tcp.expect("Should have a TCP stream");
        assert!(results.udp.is_none());
    }
    #[test]
    fn basic_connect() {
        let results =
            tokio_test::block_on(connect("127.0.0.1:3883".parse::<ServerInfo>().unwrap()))
                .expect("should be able to connect");
        results.tcp.expect("Should have a TCP stream");
        assert!(results.udp.is_some());
    }

    #[test]
    fn sync_connect() {
        use crate::buffer_unbuffer::buffer::BufferTo;

        let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();

        let mut sock = make_tcp_socket(addr.clone()).expect("failure making the socket");
        // sock.connect(&SockAddr::from(&addr)).unwrap();

        let cookie = CookieData::make_cookie();
        let mut send_buf = BytesMut::with_capacity(cookie.required_buffer_size());
        cookie.buffer_to(&mut send_buf).unwrap();
        sock.write_all(&send_buf.freeze()).unwrap();

        let mut read_buf = vec![0u8; CookieData::constant_buffer_size()];
        sock.read_exact(&mut read_buf).unwrap();
        let mut read_buf = Bytes::from(read_buf);
        let parsed_cookie: CookieData = UnbufferFrom::unbuffer_from(&mut read_buf).unwrap();
        check_ver_nonfile_compatible(parsed_cookie.version).unwrap();
    }
}
