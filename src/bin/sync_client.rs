// Copyright 2018, Collabora, Ltd.
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

use bytes::Bytes;
use std::net::{SocketAddr, TcpStream};
use vrpn::{
    constants::MAGIC_DATA,
    cookie::check_ver_nonfile_compatible,
    handler::{HandlerCode, TypedHandler},
    sync_io::{read_cookie, write_cookie, EndpointSyncTcp},
    tracker::PoseReport,
    CookieData, Message, Result, TypeDispatcher, Unbuffer,
};

#[derive(Debug)]
struct TrackerHandler {}
impl TypedHandler for TrackerHandler {
    type Item = PoseReport;
    fn handle_typed(&mut self, msg: &Message<PoseReport>) -> Result<HandlerCode> {
        println!("{:?}\n   {:?}", msg.header, msg.body);
        Ok(HandlerCode::ContinueProcessing)
    }
}

fn main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();
    let mut stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;

    // We first write our cookie, then read and check the server's cookie, before the loop.
    write_cookie(&mut stream, CookieData::from(MAGIC_DATA))?;
    let cookie_buf = read_cookie(&mut stream)?;
    let mut cookie_buf = Bytes::from(&cookie_buf[..]);

    CookieData::unbuffer_ref(&mut cookie_buf)
        .and_then(|msg| check_ver_nonfile_compatible(msg.version))?;

    let mut endpoint = EndpointSyncTcp::new(stream);
    let mut dispatcher = TypeDispatcher::new();
    let _ = dispatcher.add_typed_handler(Box::new(TrackerHandler {}), None)?;

    loop {
        endpoint.poll_endpoint(&mut dispatcher)?;
        // Every time we get here, tehre is no more messages buffered for us.
    }
}
