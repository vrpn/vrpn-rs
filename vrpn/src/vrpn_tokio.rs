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
    connection::translationtable::TranslationTable,
    error::ConnectError,
    prelude::*,
    *,
};
use bytes::{Bytes, BytesMut};
use tokio::{
    codec::{Decoder, Encoder},
    io,
    net::TcpStream,
    prelude::*,
};

fn make_tcp_socket(addr: std::net::SocketAddr) -> io::Result<std::net::TcpStream> {
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
) -> impl Future<Item = tokio::net::TcpStream, Error = ConnectError> {
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
                .and_then(|()| Ok(stream))
        })
}

pub struct FramedMessageCodec {}
impl Decoder for FramedMessageCodec {
    type Item = GenericMessage;
    type Error = unbuffer::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<GenericMessage>, Self::Error> {
        let initial_len = buf.len();
        let mut temp_buf = BytesMut::clone(buf).freeze();
        let combined_size = u32::unbuffer_ref(&mut temp_buf)
            .map_exactly_err_to_at_least()?
            .data() as usize;
        let size = MessageSize::from_unpadded_message_size(combined_size);
        if initial_len < size.padded_message_size() {
            return Ok(None);
        }
        let mut temp_buf = BytesMut::clone(buf).freeze();
        match GenericMessage::unbuffer_ref(&mut temp_buf) {
            Ok(Output(v)) => {
                buf.advance(initial_len - temp_buf.len());
                Ok(Some(v))
            }
            Err(unbuffer::Error::NeedMoreData(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Encoder for FramedMessageCodec {
    type Error = buffer::Error;
    type Item = GenericMessage;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.required_buffer_size());
        item.buffer_ref(dst)
    }
}
// pub fn handle_tcp_connection(socket: TcpStream) {
//     let sock = make_tcp_socket(addr).expect("failure making the socket");

//     let stream_future = TcpStream::connect_std(sock, &addr, &tokio::reactor::Handle::default());
//     stream_future
//         .or_else(|e| {
//             eprintln!("connect error {}", e);
//             future::err(ConnectError::IoError(e))
//         })
//         .and_then(|stream| {
//             let cookie = CookieData::from(constants::MAGIC_DATA);
//             BytesMut::new()
//                 .allocate_and_buffer(cookie)
//                 .map_err(convert_err)
//                 .into_future()
//                 .and_then(|buf| io::write_all(stream, buf.freeze()).map_err(convert_err))
//         })
//         .and_then(|(stream, _)| {
//             io::read_exact(stream, vec![0u8; CookieData::constant_buffer_size()])
//                 .map_err(convert_err)
//         })
//         .and_then(|(stream, read_buf)| {
//             println!("{:?}", stream);
//             let mut read_buf = Bytes::from(read_buf);
//             CookieData::unbuffer_ref(&mut read_buf)
//                 .map_err(|e| ConnectError::UnbufferError(e))
//                 .and_then(|Output(parsed)| {
//                     check_ver_nonfile_compatible(parsed.version).map_err(convert_err)
//                 })
//                 // TODO can pack log description here if we're enabling remote logging.
//                 // TODO if we have permission to use UDP, open an incoming socket and notify the other end about it here.
//                 .and_then(|()| Ok(stream))
//         })
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_connect() {
        let addr = "127.0.0.1:3883".parse().unwrap();
        connect_tcp(addr).wait().unwrap();
    }
}
