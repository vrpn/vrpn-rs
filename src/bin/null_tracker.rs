// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// Null tracker server: provides a tracker at Tracker0@localhost
// that just reports the identity transform on a regular basis.

extern crate futures;
extern crate tokio;
extern crate vrpn;

use std::{sync::Arc, time::Duration};
use tokio::time::Interval;
use vrpn::{
    tracker::PoseReport, ClassOfService, Error, LocalId, Quat, Result, SenderId, Sensor,
    StaticSenderName, Vec3,
};
#[derive(Debug)]
struct ConnectionAndServer {
    connection: Arc<ConnectionIp>,
    // conn_stream: ConnectionIpStream,
    interval: Interval,
    sender: LocalId<SenderId>,
}

impl ConnectionAndServer {
    fn new(connection: Arc<ConnectionIp>) -> Result<ConnectionAndServer> {
        let sender = connection.register_sender(StaticSenderName(b"Tracker0"))?;
        // let conn_stream = ConnectionIpStream::new(Arc::clone(&connection));
        Ok(ConnectionAndServer {
            connection,
            // conn_stream,
            interval: Interval::new_interval(Duration::from_millis(500)),
            sender,
        })
    }
}

impl Future for ConnectionAndServer {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if drain_poll_fn(|| self.connection.poll_endpoints())?.is_ready() {
            return Ok(Async::Ready(()));
        }
        if self
            .interval
            .poll()
            .map_err(|e| Error::OtherMessage(e.to_string()))?
            .is_ready()
        {
            // OK, send a report.
            let pose = PoseReport {
                sensor: Sensor(0),
                pos: Vec3::new(0.0, 0.0, 0.0),
                quat: Quat::new(1.0, 0.0, 0.0, 0.0),
            };
            self.connection.pack_message_body(
                None,
                self.sender,
                pose,
                ClassOfService::LOW_LATENCY,
            )?;
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
struct NullTracker {
    connection: Arc<ConnectionIp>,
    interval: Interval,
    sender: LocalId<SenderId>,
}

impl NullTracker {
    fn new(connection: Arc<ConnectionIp>) -> Result<NullTracker> {
        let sender = connection.register_sender(StaticSenderName(b"Tracker0"))?;
        Ok(NullTracker {
            connection,
            interval: Interval::new_interval(Duration::from_millis(500)),
            sender,
        })
    }
}

impl Future for NullTracker {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while self
            .interval
            .poll()
            .map_err(|e| Error::OtherMessage(e.to_string()))?
            .is_ready()
        {
            // OK, send a report.
            let pose = PoseReport {
                sensor: Sensor(0),
                pos: Vec3::new(0.0, 0.0, 0.0),
                quat: Quat::new(1.0, 0.0, 0.0, 0.0),
            };
            self.connection.pack_message_body(
                None,
                self.sender,
                pose,
                ClassOfService::LOW_LATENCY,
            )?;
        }
        Ok(Async::NotReady)
    }
}

fn main() -> Result<()> {
    let connection = ConnectionIp::new_server(None, None)?;
    let connection_stream = ConnectionIpStream::new(Arc::clone(&connection));
    let server = NullTracker::new(Arc::clone(&connection))?;
    let acceptor_stream = ConnectionIpAcceptor::new(Arc::downgrade(&connection), None)?;

    tokio::run(
        Future::select(
            Stream::select(connection_stream, acceptor_stream).drain(),
            server,
        )
        .map(|_| ())
        .map_err(|e| {
            eprintln!("error {:?}", e);
        }),
    );
    Ok(())
}
