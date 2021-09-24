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

use async_std::io;
use async_std::{
    io::BufReader,
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{AsyncBufReadExt, AsyncRead, AsyncReadExt};
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
        CookieData::make_cookie().buffer_to(&mut buf)?;
        let cookie_buf = buf.split(); // BytesMut::allocate_and_buffer(CookieData::make_cookie())?;
        stream.write_all(&cookie_buf[..]).await?;
        println!("wrote cookie");
    }
    {
        let mut cookie_buf = read_cookie(&mut stream, &mut buf).await?;
        eprintln!("{:?}", String::from_utf8_lossy(cookie_buf.chunk()));
        println!("read cookie");
        let msg = CookieData::unbuffer_from(&mut cookie_buf)?;
        check_ver_nonfile_compatible(msg.version)?;
    }

    // let mut reader = BufReader::new(&stream);
    // let mut endpoint = EndpointSyncTcp::new(stream);
    loop {
        // Read the message header and padding
        // let mut buf = BytesMut::new();
        buf.clear();

        let mut header_buf = [0u8; 24];
        AsyncReadExt::read_exact(&mut stream, &mut header_buf)
            .await
            .unwrap();
        println!("Got header");
        buf.put(&header_buf[..]);
        // Peek the size field, to compute the MessageSize.
        let size = {
            let total_len = u32::unbuffer_from(&mut buf.clone().split().freeze()).unwrap();
            MessageSize::try_from_length_field(total_len).unwrap()
        };
        println!("reading body");

        let body_stream =
            AsyncReadExt::take(stream.clone(), size.padded_body_size().try_into().unwrap());
            body_stream.read_to_end(buf)
        // Read the body of the message
        loop {
            let n = AsyncReadExt::read(&mut reader, &mut buf).await?;
            println!("n = {}, len = {}", n, buf.len());
        }
        // // let mut body_buf = [0u8; size.padded_body_size()];
        // body_buf.resize(size.padded_body_size(), 0);
        // stream.read_exact(body_buf.as_mut()).await.unwrap();
        // println!("Got body");
        // // Combine the body with the header
        // let mut full_buf = BytesMut::with_capacity(size.padded_message_size());
        // full_buf.extend_from_slice(&buf[..]);
        // full_buf.extend_from_slice(&body_buf[..]);
        // // let mut buf =  Bytes::from(
        // // async_std::io::ReadExt::chain(, ));
        // // buf.extend_from_slice(&body_buf[..]);
        // let mut full_buf = full_buf.freeze();

        // // Unbuffer the message.
        // let unbuffered = SequencedGenericMessage::try_read_from_buf(&mut full_buf).unwrap();
        // eprintln!("{:?}", unbuffered.into_inner());
    }
}
fn main() {
    // let main_task = task::spawn(async {
    //     match async_main().await {
    //         Ok(_) => todo!(),
    //         Err(_) => todo!(),
    //     }
    // });
    task::block_on(async_main()).unwrap()
}
