// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::{
    io,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    time::Duration,
};

use async_std::{
    future::{timeout, TimeoutError},
    net::{TcpListener, TcpStream, UdpSocket},
    task,
};
use bytes::{BufMut, Bytes, BytesMut};
use socket2::{SockAddr, SockRef};

use crate::{Result, Scheme, ServerInfo, VrpnError};

use super::cookie::{read_and_check_nonfile_cookie, send_nonfile_cookie};

pub struct ConnectResults {
    pub(crate) server_info: ServerInfo,
    pub(crate) tcp: TcpStream,
    pub(crate) udp: Option<UdpSocket>,
}

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

async fn make_udp_socket() -> io::Result<UdpSocket> {
    let any = std::net::Ipv4Addr::new(0, 0, 0, 0);
    let addr = SocketAddr::new(IpAddr::V4(any), 0);
    let sock = UdpSocket::bind(addr).await?;
    {
        let sock = SockRef::from(&sock);
        sock.set_reuse_address(true)?;
        sock.set_nonblocking(true)?;
        sock.set_nodelay(true)?;
    }
    Ok(sock)
}
/// Connect members that only are populated for UDP connections.
#[derive(Debug)]
pub(crate) struct UdpConnect {
    udp: UdpSocket,
    lobbed_buf: Bytes,
}
async fn outgoing_tcp_connect(addr: std::net::SocketAddr) -> Result<TcpStream> {
    let sock = make_tcp_socket(addr)?;
    sock.connect(&SockAddr::from(addr))?;
    Ok(TcpStream::from(std::net::TcpStream::from(sock)))
}

async fn lobbing(
    udp: &UdpSocket,
    buf: &Bytes,
    tcp_listener: &TcpListener,
    server: ServerInfo,
) -> std::result::Result<Option<(TcpStream, SocketAddr)>, io::Error> {
    udp.send_to(buf, server.socket_addr).await?;
    match timeout(
        Duration::from_millis(MILLIS_BETWEEN_ATTEMPTS),
        tcp_listener.accept(),
    )
    .await
    {
        Ok(result) => Ok(Some(result?)),
        Err(_) => Ok(None),
    }
}

async fn handshake(
    server_info: ServerInfo,
    tcp: TcpStream,
    udp: Option<UdpSocket>,
) -> Result<ConnectResults> {
    let mut tcp = tcp;
    send_nonfile_cookie(&mut tcp).await?;
    read_and_check_nonfile_cookie(&mut tcp).await?;
    Ok(ConnectResults {
        server_info,
        tcp,
        udp,
    })
}

async fn connect_tcp_and_udp(server: ServerInfo) -> Result<ConnectResults> {
    let udp = make_udp_socket().await?;
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
    for _ in 0..5 {
        if let Some((tcp_stream, _)) =
            lobbing(&udp, &lobbed_buf, &tcp_listener, server.clone()).await?
        {
            return handshake(server, tcp_stream, Some(udp)).await;
        }
    }
    Err(VrpnError::CouldNotConnect)
}
async fn connect_tcp_only(server: ServerInfo) -> Result<ConnectResults> {
    let tcp = outgoing_tcp_connect(server.socket_addr).await?;
    return handshake(server, tcp, None).await;
}

const MILLIS_BETWEEN_ATTEMPTS: u64 = 500;
pub async fn connect(server: ServerInfo) -> Result<ConnectResults> {
    match server.scheme {
        Scheme::UdpAndTcp => connect_tcp_and_udp(server).await,
        Scheme::TcpOnly => connect_tcp_only(server).await,
    }
}
