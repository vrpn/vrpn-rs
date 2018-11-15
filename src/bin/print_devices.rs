// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// Rough port of the vrpn_print_devices client from the
// mainline C++ VRPN repo

extern crate tokio;
#[macro_use]
extern crate vrpn;

#[macro_use]
extern crate futures;

use std::sync::Arc;
use tokio::prelude::*;
use vrpn::{
    handler::{HandlerCode, TypedHandler},
    prelude::*,
    tracker::PoseReport,
    vrpn_tokio::{
        connect_tcp, drain_poll_fn, drain_stream, ping, ConnectionIp, ConnectionIpStream, Drain,
        StreamExtras,
    },
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

type Selected = Drain<stream::Select<ConnectionIpStream, ping::Client<ConnectionIp>>>;

// struct ConnectionAndPings {
//     connection: ConnectionIpStream,
//     ping: ping::Client<ConnectionIp>,
// }

// impl ConnectionAndPings {
//     fn new(connection: Arc<ConnectionIp>, ping: ping::Client<ConnectionIp>) -> ConnectionAndPings {
//         ConnectionAndPings {
//             connection: ConnectionIpStream::new(connection),
//             ping,
//         }
//     }
// }

// impl Future for ConnectionAndPings {
//     type Item = ();
//     type Error = Error;
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         try_drain!(self.connection.poll());
//         try_drain!(self.ping.poll());
//         Ok(Async::NotReady)
//     }
// }
// enum ConnectState {
//     Handshaking(Box<dyn Future<Item = (Arc<ConnectionIp>, Selected), Error = Error>>),
//     Running(futures::Fuse<Selected>),
//     Done,
// }
// struct ClientFuture {
//     state: ConnectState,
//     connection: Option<Arc<ConnectionIp>>,
// }
// impl ClientFuture {
//     fn new(addr: std::net::SocketAddr) -> ClientFuture {
//         let fut = connect_tcp(addr).and_then(|stream| {
//             let connection = ConnectionIp::new_client(None, None, stream)?;
//             let sender = connection
//                 .register_sender(StaticSenderName(b"Tracker0"))
//                 .expect("should be able to register sender");
//             let handler_handle =
//                 connection.add_typed_handler(Box::new(TrackerHandler {}), Some(sender))?;
//             connection.pack_all_descriptions()?;
//             let ping_client = ping::Client::new(sender, Arc::clone(&connection))?;

//             let selected: Selected = ConnectionIpStream::new(Arc::clone(&connection))
//                 .select(ping_client)
//                 .drain();
//             // Ok(ConnectionIpStream::new(connection).select(ping_client))
//             // Ok(ConnectionAndPings::new(connection, ping_client))
//             Ok((connection, selected))
//         });
//         ClientFuture {
//             state: ConnectState::Handshaking(Box::new(fut)),
//             connection: None,
//         }
//     }
// }
// impl Future for ClientFuture {
//     type Item = ();
//     type Error = Error;
//     fn poll(&mut self) -> Poll<(), Error> {
//         loop {
//             let new_state = match &mut self.state {
//                 ConnectState::Handshaking(fut) => {
//                     let (connection, selected) = try_ready!(fut.poll());
//                     self.connection = Some(connection);
//                     ConnectState::Running(selected.fuse())
//                 }
//                 ConnectState::Running(fut) => {
//                     let _ = try_ready!(fut.poll());
//                     // If the connection is ready, we're all done
//                     eprintln!("Connection has returned, we are all done.");
//                     self.connection = None;
//                     ConnectState::Done
//                 }
//                 ConnectState::Done => return Ok(Async::Ready(()),
//             };
//             self.state = new_state;
//         }
//     }
// }
fn main() {
    let addr = "127.0.0.1:3883".parse().unwrap();
    let connection_future = connect_tcp(addr)
        .and_then(|stream| {
            let connection = ConnectionIp::new_client(None, None, stream)?;
            let sender = connection
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            let handler_handle =
                connection.add_typed_handler(Box::new(TrackerHandler {}), Some(sender))?;
            connection.pack_all_descriptions()?;
            let ping_client = ping::Client::new(sender, Arc::clone(&connection))?;

            let selected: Selected = ConnectionIpStream::new(Arc::clone(&connection))
                .select(ping_client)
                .drain();
            // Ok(ConnectionIpStream::new(connection).select(ping_client))
            // Ok(ConnectionAndPings::new(connection, ping_client))
            Ok(selected)
        })
        .flatten()
        .map_err(|e| {
            eprintln!("Error: {}", e);
            ()
        });
    // .wait()
    // .unwrap()
    // .wait()
    // .unwrap();

    tokio::run(connection_future);
}
