// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    base::{
        cookie::{self, check_ver_nonfile_compatible, CookieData},
        message::{GenericMessage, Message},
        types::{LocalId, RemoteId, SenderId},
    },
    buffer::{
        buffer, message::MessageSize, unbuffer, Buffer, BufferSize, ConstantBufferSize, Output,
        Unbuffer,
    },
    codec::{apply_message_framing, MessageFramed},
    connection::translationtable::TranslationTable,
    error::ConnectError,
    prelude::*,
    *,
};
use bytes::{Bytes, BytesMut};
use tokio::{
    codec::{Decoder, Encoder, Framed},
    io,
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};

pub fn make_tcp_socket(addr: std::net::SocketAddr) -> io::Result<std::net::TcpStream> {
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

fn convert_err<T>(e: T) -> ConnectError
where
    T: std::error::Error,
    ConnectError: From<T>,
{
    From::from(e)
}

pub fn connect_tcp(
    addr: std::net::SocketAddr,
) -> impl Future<Item = MessageFramed<tokio::net::TcpStream>, Error = ConnectError> {
    let sock = make_tcp_socket(addr).expect("failure making the socket");

    let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
    stream_future
        .or_else(|e| {
            eprintln!("connect error {}", e);
            future::err(ConnectError::IoError(e))
        })
        .and_then(|stream| {
            let cookie = CookieData::from(constants::MAGIC_DATA);
            BytesMut::new()
                .allocate_and_buffer(cookie)
                .map_err(convert_err)
                .into_future()
                .and_then(|buf| io::write_all(stream, buf.freeze()).map_err(convert_err))
        })
        .and_then(|(stream, _)| {
            io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()])
                .map_err(convert_err)
        })
        .and_then(|(stream, read_buf)| {
            println!("{:?}", stream);
            let mut read_buf = Bytes::from(read_buf);
            CookieData::unbuffer_ref(&mut read_buf)
                .map_err(|e| ConnectError::UnbufferError(e))
                .and_then(|Output(parsed)| {
                    check_ver_nonfile_compatible(parsed.version).map_err(convert_err)
                })
                // TODO can pack log description here if we're enabling remote logging.
                // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
                .and_then(|()| Ok(apply_message_framing(stream)))
        })
}

pub fn handle_tcp_connection(
    socket: TcpStream,
) -> impl Future<Item = MessageFramed<tokio::net::TcpStream>, Error = ConnectError> {
    io::read_exact(socket, vec![0u8; CookieData::constant_buffer_size()])
        .map_err(convert_err)
        .and_then(|(stream, read_buf)| {
            println!("{:?}", stream);
            let mut read_buf = Bytes::from(read_buf);
            CookieData::unbuffer_ref(&mut read_buf)
                .map_err(|e| ConnectError::UnbufferError(e))
                .and_then(|Output(parsed)| {
                    check_ver_nonfile_compatible(parsed.version).map_err(convert_err)
                })
                .and_then(|()| Ok(stream))
        })
        .and_then(|stream| {
            let cookie = CookieData::from(constants::MAGIC_DATA);
            BytesMut::new()
                .allocate_and_buffer(cookie)
                .map_err(convert_err)
                .into_future()
                .and_then(|buf| io::write_all(stream, buf.freeze()).map_err(convert_err))
        })
        .and_then(|(stream, _)| {
            // TODO can pack log description here if we're enabling remote logging.
            // TODO should send descriptions here.
            Ok(apply_message_framing(stream))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_connect() {
        let addr = "127.0.0.1:3883".parse().unwrap();
        connect_tcp(addr).wait().unwrap();
    }
}
