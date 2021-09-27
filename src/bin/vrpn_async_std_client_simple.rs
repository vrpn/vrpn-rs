// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! A simple, synchronous-IO client for testing purposes.
//! Doesn't use any of the async-io stuff in the vrpn crate,
//! so this is durable even if Tokio totally changes everything.
//!
//! However, this doesn't use any Connection structs - just an endpoint and a type dispatcher.
//! In normal usage, this would be bundled into a Connection.
extern crate bytes;
extern crate vrpn;

use async_std::{
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use bytes::{Buf, BytesMut};

use futures::AsyncReadExt;
use vrpn::buffer_unbuffer::BufferUnbufferError;

use vrpn::{
    buffer_unbuffer::{BufferTo, UnbufferFrom},
    data_types::{
        cookie::check_ver_nonfile_compatible, CookieData, MessageSize, SequencedGenericMessage,
    },
    vrpn_async_std::read_cookie,
    Result,
};

async fn async_main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();
    let mut stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    let mut buf = BytesMut::with_capacity(2048);

    // We first write our cookie, then read and check the server's cookie, before the loop.
    {
        buf.clear();
        CookieData::make_cookie().buffer_to(&mut buf)?;
        let cookie_buf = buf.split();
        stream.write_all(&cookie_buf[..]).await?;
        println!("wrote cookie");
    }
    {
        buf.clear();
        read_cookie(&mut stream, &mut buf).await?;
        let mut cookie_buf = buf.split();
        eprintln!("{:?}", String::from_utf8_lossy(cookie_buf.chunk()));
        println!("read cookie");
        let msg = CookieData::unbuffer_from(&mut cookie_buf)?;
        check_ver_nonfile_compatible(msg.version)?;
        cookie_buf.unsplit(buf);
        buf = cookie_buf;
    }

    loop {
        buf.clear();
        let msg = try_get_message(&mut stream, &mut buf).await?;
        eprintln!("{:?}", msg.into_inner());
    }
}

fn try_decode(bytes_mut: &mut BytesMut) -> Result<Option<SequencedGenericMessage>> {
    let mut existing_bytes = std::io::Cursor::new(&*bytes_mut);

    match SequencedGenericMessage::try_read_from_buf(&mut existing_bytes) {
        Ok(sgm) => {
            // consume the bytes from the original buffer.
            let consumed = bytes_mut.remaining() - existing_bytes.remaining();
            bytes_mut.advance(consumed);
            println!(
                "consumed {} bytes, {} remain in buffer",
                consumed,
                bytes_mut.remaining()
            );
            println!("{:?}", bytes_mut);
            Ok(Some(sgm))
        }
        Err(BufferUnbufferError::NeedMoreData(requirement)) => {
            println!("need more data: {} bytes", requirement);
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

async fn try_read_header(stream: &mut TcpStream, bytes_mut: &mut BytesMut) -> Result<MessageSize> {
    assert!(bytes_mut.is_empty());
    let mut header_buf = [0u8; 24];
    AsyncReadExt::read_exact(stream, &mut header_buf).await?;
    let size = {
        let mut size_buf = std::io::Cursor::new(&header_buf);
        let total_len = u32::unbuffer_from(&mut size_buf).unwrap();
        MessageSize::try_from_length_field(total_len).unwrap()
    };
    bytes_mut.extend_from_slice(&header_buf[..]);
    println!("Got header {:?}", size);
    Ok(size)
}

async fn try_get_message(
    stream: &mut (impl AsyncReadExt + Unpin),
    bytes_mut: &mut BytesMut,
) -> Result<SequencedGenericMessage> {
    loop {
        match try_decode(bytes_mut) {
            Ok(Some(msg)) => return Ok(msg),
            Ok(None) => {
                let mut body_buf = [0u8; 2048];
                let n = AsyncReadExt::read(stream, &mut body_buf).await?;
                println!("Read {} bytes from stream", n);
                bytes_mut.extend_from_slice(&body_buf[..n]);

                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}
fn main() {
    task::block_on(async_main()).unwrap()
}
