// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    async_io::cookie::{read_and_check_nonfile_cookie, send_nonfile_cookie},
    Error,
};
use std::net::SocketAddr;
use tokio::{io, net::TcpStream, prelude::*};

pub fn make_tcp_socket(addr: SocketAddr) -> io::Result<std::net::TcpStream> {
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

fn outgoing_tcp_connect(
    addr: std::net::SocketAddr,
) -> impl Future<Item = tokio::net::TcpStream, Error = Error> {
    make_tcp_socket(addr)
        .map_err(|e| Error::from(e))
        .into_future()
        .and_then(move |sock| {
            let addr = addr.clone();
            TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default())
                .map_err(|e| Error::from(e))
        })
}

pub fn outgoing_handshake<T>(socket: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncRead + AsyncWrite,
{
    send_nonfile_cookie(socket).and_then(read_and_check_nonfile_cookie)
    // TODO can pack log description here if we're enabling remote logging.
    // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
}

pub fn connect_tcp(
    addr: std::net::SocketAddr,
) -> impl Future<Item = tokio::net::TcpStream, Error = Error> {
    outgoing_tcp_connect(addr).and_then(outgoing_handshake)
    // TODO can pack log description here if we're enabling remote logging.
    // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
}

pub fn incoming_handshake<T>(socket: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncRead + AsyncWrite,
{
    // If connection is incoming
    read_and_check_nonfile_cookie(socket).and_then(send_nonfile_cookie)

    // TODO can pack log description here if we're enabling remote logging.
    // TODO should send descriptions here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use crate::{
        constants::MAGIC_DATA, cookie::check_ver_nonfile_compatible, ConstantBufferSize,
        CookieData, Unbuffer,
    };

    #[test]
    fn basic_connect() {
        let addr = "127.0.0.1:3883".parse().unwrap();
        connect_tcp(addr).wait().unwrap();
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
