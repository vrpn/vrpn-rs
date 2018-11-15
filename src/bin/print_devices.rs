// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// Rough port of the vrpn_print_devices client from the
// mainline C++ VRPN repo

extern crate tokio;
extern crate vrpn;

#[macro_use]
extern crate futures;

use std::sync::Arc;
use tokio::prelude::*;
use vrpn::{
    handler::{HandlerCode, TypedHandler},
    prelude::*,
    tracker::PoseReport,
    vrpn_tokio::{connect_tcp, ping, ConnectionIp, ConnectionIpStream},
    Error, Message, Result, StaticSenderName,
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
// struct ConnectionAndPings {
//     connection: Arc<ConnectionIp>,
//     pings: Vec<ping::Client<ConnectionIp>>,
//     interval: Interval,
// }

// impl ConnectionAndPings {
//     fn new(connection: Arc<ConnectionIp>) -> ConnectionAndPings {
//         ConnectionAndPings {
//             connection,
//             pings: Vec::new(),
//             interval: Interval::new_interval(Duration::from_secs(1)),
//         }
//     }
//     fn start_ping(&mut self, sender: LocalId<SenderId>) -> Result<()> {
//         self.pings
//             .push(ping::Client::new(sender, Arc::clone(&self.connection))?);
//         Ok(())
//     }
// }

// impl Future for ConnectionAndPings {
//     type Item = ();
//     type Error = Error;
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         if self
//             .interval
//             .poll()
//             .map_err(|e| Error::OtherMessage(e.to_string()))?
//             .is_ready()
//         {
//             for ping_client in &mut self.pings {
//                 if let Some(radio_silence) = ping_client.check_ping_cycle()? {
//                     eprintln!(
//                         "It has been {} since the first unanwered ping was sent to the server!",
//                         radio_silence
//                     );
//                 }
//             }
//         }
//         self.connection.poll_endpoints()
//     }
// }

struct ConnectionAndPings {
    connection: ConnectionIpStream,
    ping: ping::Client<ConnectionIp>,
}

impl ConnectionAndPings {
    fn new(connection: Arc<ConnectionIp>, ping: ping::Client<ConnectionIp>) -> ConnectionAndPings {
        ConnectionAndPings {
            connection: ConnectionIpStream::new(connection),
            ping,
        }
    }
}

impl Future for ConnectionAndPings {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match try_ready!(self.connection.poll()) {
            Some(()) => {
                task::current().notify();
            }
            None => {
                return Ok(Async::Ready(()));
            }
        }
        match try_ready!(self.ping.poll()) {
            Some(()) => {
                task::current().notify();
            }
            None => {
                return Ok(Async::Ready(()));
            }
        }
        Ok(Async::NotReady)
    }
}
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    connect_tcp(addr)
        .and_then(|stream| {
            let connection = ConnectionIp::new_client(None, None, stream)?;
            let sender = connection
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            let handler_handle =
                connection.add_typed_handler(Box::new(TrackerHandler {}), Some(sender))?;
            connection.pack_all_descriptions()?;
            let ping_client = ping::Client::new(sender, Arc::clone(&connection))?;

            // Ok(ConnectionIpStream::new(connection).select(ping_client))
            Ok(ConnectionAndPings::new(connection, ping_client))
        })
        .map_err(|e| {
            eprintln!("Error: {}", e);
            ()
        })
        .wait()
        .unwrap()
        .wait()
        .unwrap();

    //tokio::run(connection_stream);
}
