// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{connection::*, vrpn_tokio::endpoint_ip::EndpointIp, Error, LogFileNames, Result};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream},
    prelude::*,
};

#[derive(Debug)]
pub struct ConnectionIp {
    core: ConnectionCore<EndpointIp>,
    server_tcp: Option<TcpListener>,
}

impl ConnectionIp {
    /// Create a new ConnectionIp that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        addr: Option<SocketAddr>,
    ) -> Result<Arc<ConnectionIp>> {
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let server_tcp = TcpListener::bind(&addr)?;
        Ok(Arc::new(ConnectionIp {
            core: ConnectionCore::new(Vec::new(), local_log_names, None),
            server_tcp: Some(server_tcp),
        }))
    }

    /// Create a new ConnectionIp that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> Result<Arc<ConnectionIp>> {
        let mut endpoints: Vec<Option<EndpointIp>> = Vec::new();
        endpoints.push(Some(EndpointIp::new(reliable_channel)));
        Ok(Arc::new(ConnectionIp {
            core: ConnectionCore::new(endpoints, local_log_names, remote_log_names),
            server_tcp: None,
        }))
    }

    pub fn poll_endpoints(&self) -> Poll<(), Error> {
        eprintln!("in poll_endpoints");
        let endpoints = self.endpoints();
        let dispatcher = self.dispatcher();
        {
            let mut endpoints = endpoints.lock()?;
            let mut dispatcher = dispatcher.lock()?;
            let mut got_not_ready = false;
            for ep in endpoints.iter_mut().flatten() {
                match ep.poll_endpoint(&mut dispatcher)? {
                    Async::Ready(()) => {
                        eprintln!("endpoint closed apparently");
                        // that endpoint closed.
                        // TODO Handle this
                    }
                    Async::NotReady => {
                        got_not_ready = true;
                        // this is normal.
                    }
                }
            }
            if got_not_ready {
                Ok(Async::NotReady)
            } else {
                Ok(Async::Ready(()))
            }
        }
    }
}

impl Connection for ConnectionIp {
    type SpecificEndpoint = EndpointIp;
    fn connection_core(&self) -> &ConnectionCore<Self::SpecificEndpoint> {
        &self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handler::{HandlerCode, TypedHandler},
        tracker::*,
        Message, SomeId, StaticSenderName, StaticTypeName, TypeSafeId,
    };
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Debug)]
    struct TrackerHandler {
        flag: Arc<Mutex<bool>>,
    }
    impl TypedHandler for TrackerHandler {
        type Item = PoseReport;
        fn handle_typed(&mut self, msg: &Message<PoseReport>) -> Result<HandlerCode> {
            println!("{:?}", msg);
            let mut flag = self.flag.lock()?;
            *flag = true;
            Ok(HandlerCode::ContinueProcessing)
        }
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker() {
        use crate::vrpn_tokio::connect_tcp;
        let addr = "127.0.0.1:3883".parse().unwrap();
        let flag = Arc::new(Mutex::new(false));

        connect_tcp(addr)
            .and_then(|stream| -> Result<()> {
                let conn = ConnectionIp::new_client(None, None, stream)?;
                let sender = conn
                    .register_sender(StaticSenderName(b"Tracker0"))
                    .expect("should be able to register sender");
                let handler_handle = conn.add_typed_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    SomeId(sender),
                )?;
                conn.pack_all_descriptions()?;
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                conn.remove_handler(handler_handle)
                    .expect("should be able to remove handler");
                Ok(())
            })
            .wait()
            .unwrap();
        assert!(*flag.lock().unwrap() == true);
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker_manual() {
        use crate::vrpn_tokio::connect_tcp;
        let addr = "127.0.0.1:3883".parse().unwrap();
        let flag = Arc::new(Mutex::new(false));

        connect_tcp(addr)
            .and_then(|stream| {
                let conn = ConnectionIp::new_client(None, None, stream)?;
                let tracker_message_id = conn
                    .register_type(StaticTypeName(b"vrpn_Tracker Pos_Quat"))
                    .expect("should be able to register type");
                let sender = conn
                    .register_sender(StaticSenderName(b"Tracker0"))
                    .expect("should be able to register sender");
                conn.add_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    SomeId(tracker_message_id),
                    SomeId(sender),
                )?;
                conn.pack_all_descriptions()?;
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                Ok(())
                // Ok(future::poll_fn(move || {
                //     eprintln!("polling");
                //     conn.poll_endpoints()
                // })
                // .timeout(Duration::from_secs(4))
                // .map(|_| ()))
            })
            .wait()
            .unwrap();
        assert!(*flag.lock().unwrap() == true);
    }
}
