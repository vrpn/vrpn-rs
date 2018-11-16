// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// Rough port of the vrpn_print_devices client from the
// mainline C++ VRPN repo

extern crate futures;
extern crate tokio;
extern crate vrpn;

use std::sync::Arc;
use tokio::prelude::*;
use vrpn::{
    async_io::{connect_tcp, ping, ConnectionIp, ConnectionIpStream, Drain, StreamExtras},
    handler::{HandlerCode, TypedHandler},
    prelude::*,
    tracker::PoseReport,
    Message, Result, StaticSenderName,
};

#[derive(Debug)]
struct TrackerHandler {}
impl TypedHandler for TrackerHandler {
    type Item = PoseReport;
    fn handle_typed(&mut self, msg: &Message<PoseReport>) -> Result<HandlerCode> {
        println!("{:?}", msg);
        Ok(HandlerCode::ContinueProcessing)
    }
}

type Selected = Drain<stream::Select<ConnectionIpStream, ping::Client<ConnectionIp>>>;

fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let connection_future = connect_tcp(addr)
        .and_then(|stream| {
            let connection = ConnectionIp::new_client(None, None, stream)?;
            let sender = connection
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            let _ = connection.add_typed_handler(Box::new(TrackerHandler {}), Some(sender))?;
            connection.pack_all_descriptions()?;
            let ping_client = ping::Client::new(sender, Arc::clone(&connection))?;

            let selected: Selected = ConnectionIpStream::new(Arc::clone(&connection))
                .select(ping_client)
                .drain();
            Ok(selected)
        })
        .flatten()
        .map_err(|e| {
            eprintln!("Error: {}", e);
            ()
        });

    tokio::run(connection_future);
}
