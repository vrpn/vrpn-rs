// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

/// A simple, synchronous-IO client for testing purposes.
/// Doesn't use any of the async-io stuff in the vrpn crate,
/// so this is durable even if Tokio totally changes everything.
///
/// However, this doesn't use any Connection structs - just an endpoint and a type dispatcher.
/// In normal usage, this would be bundled into a Connection.
extern crate bytes;
extern crate vrpn;

use std::pin::Pin;

use async_std::{
    io::WriteExt,
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use bytes::{Bytes, BytesMut};
use futures::{stream::IntoAsyncRead, AsyncRead};
use vrpn::{
    buffer_unbuffer::{peek_u32, BytesMutExtras, UnbufferFrom},
    data_types::{
        cookie::check_ver_nonfile_compatible, CookieData, GenericMessage, MessageSize,
        SequencedGenericMessage, TypedMessage,
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

    // We first write our cookie, then read and check the server's cookie, before the loop.
    let cookie_buf = BytesMut::allocate_and_buffer(CookieData::make_cookie())?;
    stream.write_all(&cookie_buf[..]).await?;
    println!("wrote cookie");

    let cookie_buf = read_cookie(&mut stream).await?;
    let mut cookie_buf = Bytes::copy_from_slice(&cookie_buf[..]);
    println!("read cookie");

    let msg = CookieData::unbuffer_from(&mut cookie_buf)?;
    check_ver_nonfile_compatible(msg.version)?;

    // let mut endpoint = EndpointSyncTcp::new(stream);
    loop {
        // Read the message header and padding
        let mut buf = BytesMut::new();

        buf.resize(24, 0);
        stream.read_exact(buf.as_mut()).await.unwrap();
        println!("Got header");
        // Peek the size field, to compute the MessageSize.
        let total_len = peek_u32(&buf.clone().freeze()).unwrap();
        let size = MessageSize::try_from_length_field(total_len).unwrap();
        println!("reading body");

        // Read the body of the message
        let mut body_buf = BytesMut::new();
        body_buf.resize(size.padded_body_size(), 0);
        stream.read_exact(body_buf.as_mut()).await.unwrap();
        println!("Got body");

        // Combine the body with the header
        buf.extend_from_slice(&body_buf[..]);
        let mut buf = buf.freeze();

        // Unbuffer the message.
        let unbuffered = SequencedGenericMessage::try_read_from_buf(&mut buf).unwrap();
        eprintln!("{:?}", unbuffered.into_inner());
    }
}
fn main() -> Result<()> {
    let main_task = task::spawn(async { async_main().await });
    task::block_on(main_task)
}
