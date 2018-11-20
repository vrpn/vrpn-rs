// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{buf::IntoBuf, Buf, BufMut, Bytes, BytesMut};
use crate::prelude::*;
use crate::{
    async_io::{
        connect::{make_udp_socket, outgoing_handshake, outgoing_tcp_connect},
        connection_ip::ConnectionIpAcceptor,
        cookie::{read_and_check_nonfile_cookie, send_nonfile_cookie},
        ping, ConnectionIp, ConnectionIpStream, Drain, StreamExtras,
    },
    constants,
    cookie::check_ver_nonfile_compatible,
    ConnectionStatus, CookieData, Error, Result, Scheme, ServerInfo, Unbuffer,
};
use std::{
    fmt::{self, Debug},
    io,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    time::Duration,
};
use tk_listen::{ListenExt, SleepOnError};
use tokio::net::{
    tcp::{ConnectFuture, Incoming},
    TcpListener, TcpStream, UdpSocket,
};
use tokio::prelude::*;

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
    Lobbing(Option<TcpListener>, IpAddr),
    WaitingForConnection(WaitForConnect),
    Connecting(ConnectFuture),
    SendingHandshake,
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

/// A future that handles the connection and handshake process.
#[derive(Debug)]
pub(crate) struct Connect {
    state: Option<State>,
    server: ServerInfo,
    stream: Option<TcpStream>,
    udp_connect: Option<UdpConnect>,
    cookie_buf: <Bytes as IntoBuf>::Buf,
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
                })
            }
        }
    }

    fn poll_one(&mut self, state: &mut State) -> Poll<Option<State>, Error> {
        match state {
            State::Lobbing(tcp_listener, ip) => {
                if let Some(udp_connect) = &mut self.udp_connect {
                    try_ready!(udp_connect
                        .udp
                        .poll_send_to(&udp_connect.lobbed_buf, &self.server.socket_addr));
                    //if we don't return immediately, then we're OK.

                    return Ok(Async::Ready(Some(State::WaitingForConnection(
                        WaitForConnect::new(*ip, tcp_listener.take().unwrap()),
                    ))));
                } else {
                    return Err(Error::OtherMessage(String::from("no udp socket found?")));
                }
            }
            State::Connecting(conn_future) => {
                let stream = try_ready!(conn_future.poll());
                self.stream = Some(stream);
                return Ok(Async::Ready(Some(State::SendingHandshake)));
            }
            State::WaitingForConnection(conn_stream) => {
                let stream = try_ready!(conn_stream
                    .poll()
                    .map_err(|e| Error::OtherMessage(String::from(""))));
                self.stream = Some(stream);
                return Ok(Async::Ready(Some(State::SendingHandshake)));
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
                return Ok(Async::Ready(Some(State::ReceivingHandshake(buf))));
            }
            State::ReceivingHandshake(buf) => {
                while buf.len() < CookieData::constant_buffer_size() {
                    let _ = try_ready!(self.stream.as_mut().unwrap().read_buf(buf));
                }
                let mut buf = buf.clone().freeze();
                let cookie = CookieData::unbuffer_ref(&mut buf)?;
                check_ver_nonfile_compatible(cookie.version)?;
                return Ok(Async::Ready(None));
            }
        };
    }
}

impl Future for Connect {
    type Item = ConnectResults;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let mut old_state = self.state.take();
            match self.poll_one(old_state.as_mut().unwrap()) {
                Ok(Async::NotReady) => self.state = old_state,
                Ok(Async::Ready(Some(s))) => self.state = Some(s),
                Ok(Async::Ready(None)) => {
                    let udp = self.udp_connect.take().map(|udp_connect| udp_connect.udp);

                    return Ok(Async::Ready(ConnectResults {
                        tcp: self.stream.take(),
                        udp,
                    }));
                }
                Err(e) => Err(e)?,
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ClientInfo {
    ConnectionSetupFuture(Connect),
    Info(ServerInfo),
    Server,
}

impl ClientInfo {
    pub(crate) fn poll(&mut self, num_endpoints: usize) -> Poll<Option<ConnectResults>, Error> {
        loop {
            match self {
                ClientInfo::ConnectionSetupFuture(fut) => {
                    let result = try_ready!(fut.poll());
                    *self = ClientInfo::Info(fut.server.clone());
                    return Ok(Async::Ready(Some(result)));
                }
                ClientInfo::Info(info) => {
                    if num_endpoints == 0 {
                        *self = ClientInfo::ConnectionSetupFuture(Connect::new(info.clone())?);
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
            ClientInfo::ConnectionSetupFuture(_) => ConnectionStatus::ClientConnecting,
            ClientInfo::Info(_) => ConnectionStatus::ClientConnected,
            ClientInfo::Server => ConnectionStatus::Server(num_endpoints),
        }
    }
}
