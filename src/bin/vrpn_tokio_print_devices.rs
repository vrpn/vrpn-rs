// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// Rough port of the vrpn_print_devices client from the
// mainline C++ VRPN repo

extern crate futures;
extern crate tokio;
extern crate vrpn;

use std::sync::Arc;
use vrpn::{
    handler::{HandlerCode, TypedHandler},
    prelude::*,
    tracker::PoseReport,
    vrpn_tokio::{ping, ConnectionIp, ConnectionIpStream, StreamExtras},
    Message, Result, ServerInfo, StaticSenderName,
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

// type Selected = Drain<stream::Select<ConnectionIpStream, ping::Client<ConnectionIp>>>;

fn main() {
    let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();

    let connection =
        ConnectionIp::new_client(server, None, None).expect("should be able to create client");
    let sender = connection
        .register_sender(StaticSenderName(b"Tracker0"))
        .expect("should be able to register sender");
    let _ = connection
        .add_typed_handler(Box::new(TrackerHandler {}), Some(sender))
        .expect("should be able to add handler");
    let ping_client = ping::Client::new(sender, Arc::clone(&connection))
        .expect("should be able to create ping client");

    let selected = ConnectionIpStream::new(Arc::clone(&connection))
        .select(ping_client)
        .map_err(|e| {
            eprintln!("error: {}", e);
        })
        .drain();

    tokio::run(selected);
}
