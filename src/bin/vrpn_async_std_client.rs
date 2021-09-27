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

use std::convert::TryInto;

use async_std::io::{self, Cursor};
use async_std::{
    io::BufReader,
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{AsyncBufReadExt, AsyncRead, AsyncReadExt};
use vrpn::buffer_unbuffer::BufferUnbufferError;
use vrpn::vrpn_async_std::{read_n_into_bytes_mut, BytesMutReader};
use vrpn::{
    buffer_unbuffer::{peek_u32, BufferTo, BytesMutExtras, UnbufferFrom},
    data_types::{
        cookie::check_ver_nonfile_compatible, CookieData, MessageSize, SequencedGenericMessage,
        TypedMessage,
    },
    handler::{HandlerCode, TypedHandler},
    tracker::PoseReport,
    vrpn_async_std::read_cookie,
    Result, TypeDispatcher, VrpnError,
};

#[derive(Debug)]
struct TrackerHandler {}
impl TypedHandler for TrackerHandler {
    type Item = PoseReport;
    fn handle_typed(&mut self, msg: &TypedMessage<PoseReport>) -> Result<HandlerCode> {
        println!("{:?}\n   {:?}", msg.header, msg.body);
        Ok(HandlerCode::ContinueProcessing)
    }
}

struct MessageReader<T: AsyncRead> {
    stream: Box<T>,
    failed: bool,
}

// impl<T: AsyncRead> MessageReader<T> {
//     pub fn new(stream: T) -> Self {
//         MessageReader {
//             stream: Box::new(stream),
//             failed: false,
//         }
//     }

//     pub async fn next_message(&mut self) -> Result<GenericMessage> {
//         // Read the message header and padding
//         let mut buf = BytesMut::new();

//         buf.resize(24, 0);
//         Box::pin(self.stream).read_exact(buf.as_mut()).await?;
//         println!("Got header");
//         // Peek the size field, to compute the MessageSize.
//         let total_len = peek_u32(&buf.clone().freeze()).unwrap();
//         let size = MessageSize::try_from_length_field(total_len)?;
//         println!("reading body");

//         // Read the body of the message
//         let mut body_buf = BytesMut::new();
//         body_buf.resize(size.padded_body_size(), 0);
//         self.stream.read_exact(body_buf.as_mut()).await?;
//         println!("Got body");

//         // Combine the body with the header
//         buf.extend_from_slice(&body_buf[..]);
//         let mut buf = buf.freeze();

//         // Unbuffer the message.
//         let unbuffered = SequencedGenericMessage::try_read_from_buf(&mut buf)?;
//         Ok(unbuffered.into_inner())
//     }
// }

async fn async_main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();
    let mut stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    let mut buf = BytesMut::with_capacity(2048);

    // We first write our cookie, then read and check the server's cookie, before the loop.
    {
        buf.clear();
        CookieData::make_cookie().buffer_to(&mut buf)?;
        let cookie_buf = buf.split(); // BytesMut::allocate_and_buffer(CookieData::make_cookie())?;
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
            return Ok(Some(sgm));
        }
        Err(e) => {
            if let BufferUnbufferError::NeedMoreData(_) = e {
                return Ok(None);
            } else {
                return Err(e.into());
            }
        }
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
    stream: &mut TcpStream,
    bytes_mut: &mut BytesMut,
) -> Result<SequencedGenericMessage> {
    loop {
        match try_decode(bytes_mut) {
            Ok(Some(msg)) => return Ok(msg),
            Ok(None) => {
                let mut body_buf = [0u8; 2048];
                let n = AsyncReadExt::read(stream, &mut body_buf).await?;
                bytes_mut.extend_from_slice(&body_buf[..n]);

                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}
fn main() {
    task::block_on(async_main()).unwrap()
}
