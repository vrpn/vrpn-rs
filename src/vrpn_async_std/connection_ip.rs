// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{connection::*, data_types::log::LogFileNames, Result, ServerInfo};
use async_std::net::TcpListener;
use futures::{future::BoxFuture, FutureExt, Stream};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    task::Poll,
};

use super::{
    connect::{connect, ConnectResults},
    endpoint_ip::EndpointIp,
};

pub(crate) enum ConnectionIpInfo {
    /// This variant stores the server info for reconnecting
    ClientConnectionInfo(ServerInfo),
    /// This stores the future that connects
    ClientConnectionSetupFuture(BoxFuture<'static, Result<ConnectResults>>),
    /// This just marks us as a server
    Server,
}

impl ConnectionIpInfo {
    pub(crate) fn status(&self, num_endpoints: usize) -> ConnectionStatus {
        match self {
            ConnectionIpInfo::ClientConnectionSetupFuture(_) => ConnectionStatus::ClientConnecting,
            ConnectionIpInfo::ClientConnectionInfo(_) => ConnectionStatus::ClientConnected,
            ConnectionIpInfo::Server => ConnectionStatus::Server(num_endpoints),
        }
    }
}
pub struct ConnectionIp {
    core: ConnectionCore<EndpointIp>,
    server_tcp: Option<Mutex<TcpListener>>,
    // server_acceptor: Arc<Mutex<Option<ConnectionIpAcceptor>>>,
    client_info: Mutex<ConnectionIpInfo>,
}

const DEFAULT_PORT: u16 = 3883;

impl ConnectionIp {
    /// Create a new ConnectionIp that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        _addr: Option<SocketAddr>,
    ) -> Result<Arc<ConnectionIp>> {
        let conn = Arc::new(ConnectionIp {
            core: ConnectionCore::new(Vec::new(), local_log_names, None),
            // server_acceptor: Arc::new(Mutex::new(None)),
            // server_tcp: Some(Mutex::new(server_tcp)),
            server_tcp: None,
            client_info: Mutex::new(ConnectionIpInfo::Server),
        });
        // {
        //     let accepter = ConnectionIpAcceptor::new(Arc::downgrade(&conn), addr)?;
        //     let mut locked_acceptor = conn.server_acceptor.lock()?;
        //     *locked_acceptor = Some(accepter);
        // }
        Ok(conn)
    }

    /// Create a new ConnectionIp that is a client.
    pub fn new_client(
        server: ServerInfo,
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> Result<Arc<ConnectionIp>> {
        let endpoints: Vec<Option<EndpointIp>> = Vec::new();
        // let connect = Connect::new(server)?;
        let ret = Arc::new(ConnectionIp {
            core: ConnectionCore::new(endpoints, local_log_names, remote_log_names),
            // server_acceptor: None,
            client_info: Mutex::new(ConnectionIpInfo::ClientConnectionSetupFuture(
                connect(server).boxed(),
            )),
            server_tcp: None,
        });
        ret.send_all_descriptions()?;
        Ok(ret)
    }

    pub fn poll_endpoints(&self, cx: &mut std::task::Context<'_>) -> Poll<Result<Option<()>>> {
        // eprintln!("in <ConnectionIp as Future>::poll");
        // if let Some(listener_mutex) = &self.server_tcp {
        //     let listener = listener_mutex.lock()?;
        //     match listener.incoming().poll()? {
        //         Async::Ready(Some(sock)) => {
        //             // OK, we got a new one.
        //             let endpoints = self.endpoints();
        //             tokio::spawn(
        //                 incoming_handshake(sock)
        //                     .and_then(move |stream| {
        //                         if let Ok(mut epoints) = endpoints.lock() {
        //                             epoints.push(Some(EndpointIp::new(stream)));
        //                         }
        //                         Ok(())
        //                     })
        //                     .map_err(|e| {
        //                         eprintln!("err: {:?}", e);
        //                     }),
        //             );
        //         }
        //         Async::Ready(None) => return Ok(Async::Ready(None)),
        //         Async::NotReady => (),
        //     }
        // }

        // Connect/reconnect if needed.
        {
            let mut client_info = self.client_info.lock()?;
            let ep_arc = self.endpoints();
            let mut endpoints = ep_arc.lock()?;
            if let ConnectionIpInfo::ClientConnectionSetupFuture(f) = &mut *client_info {
                match f.as_mut().poll(cx) {
                    Poll::Ready(Ok(results)) => {
                        endpoints.push(Some(EndpointIp::new(results.tcp, results.udp)));
                        *client_info = ConnectionIpInfo::ClientConnectionInfo(results.server_info)
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            };
        }

        // let mut acceptor = self.server_acceptor.lock()?;
        // match &mut (*acceptor) {
        //     Some(a) => loop {
        //         let poll_result = a.poll()?;
        //         match poll_result {
        //             Poll::Pending => break,
        //             Poll::Ready(Some(_)) => (),
        //             Poll::Ready(None) => return Ok(Poll::Ready(None)),
        //         }
        //     },
        //     None => (),
        // }
        let endpoints = self.endpoints();
        let dispatcher = self.dispatcher();
        {
            let mut endpoints = endpoints.lock()?;
            let mut dispatcher = dispatcher.lock()?;
            let mut got_not_ready = false;
            // Go through and poll each endpoint, "taking" the ones that are closed.
            for ep in endpoints.iter_mut() {
                let ready = match ep {
                    Some(endpoint) => endpoint.poll_endpoint(&mut dispatcher, cx).is_ready(),
                    _ => true,
                };
                if ready {
                    let _ = ep.take();
                } else {
                    got_not_ready = true;
                }
            }
            // Now, retain only the non-taken endpoints in the vector.
            endpoints.retain(|ep| ep.is_some());

            if got_not_ready {
                Poll::Pending
            } else {
                Poll::Ready(Ok(Some(())))
            }
        }
    }
}

impl Connection for ConnectionIp {
    type SpecificEndpoint = EndpointIp;
    fn connection_core(&self) -> &ConnectionCore<Self::SpecificEndpoint> {
        &self.core
    }

    fn status(&self) -> ConnectionStatus {
        let ep = self.endpoints();
        let endpoints = ep.lock().unwrap();
        let info = self.client_info.lock().unwrap();
        info.status(endpoints.len())
    }
}

pub struct ConnectionIpStream {
    connection: Arc<ConnectionIp>,
}

impl ConnectionIpStream {
    pub fn new(connection: Arc<ConnectionIp>) -> ConnectionIpStream {
        ConnectionIpStream { connection }
    }
}

impl Stream for ConnectionIpStream {
    type Item = Result<()>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // eprintln!("in <ConnectionIpStream as Stream>::poll");
        self.connection.poll_endpoints(cx).map(|x| x.transpose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_types::{Message, StaticMessageTypeName, StaticSenderName, TypedMessage},
        handler::{HandlerCode, TypedHandler},
        tracker::*,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    #[derive(Debug)]
    struct TrackerHandler {
        flag: Arc<AtomicBool>,
    }
    impl TrackerHandler {
        fn new(flag: &Arc<AtomicBool>) -> Box<TrackerHandler> {
            Box::new(TrackerHandler {
                flag: Arc::clone(flag),
            })
        }
    }
    impl TypedHandler for TrackerHandler {
        type Item = PoseReport;
        fn handle_typed(&mut self, msg: &TypedMessage<PoseReport>) -> Result<HandlerCode> {
            println!("{:?}", msg);
            self.flag.store(true, Ordering::SeqCst);
            Ok(HandlerCode::ContinueProcessing)
        }
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker_tcp() {
        let flag = Arc::new(AtomicBool::new(false));
        async fn function(flag: &Arc<AtomicBool>) -> Result<()> {
            let mut cx = futures::task::Context::from_waker(futures::task::noop_waker_ref());
            let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>()?;
            let conn = ConnectionIp::new_client(server, None, None)?;
            let sender = conn
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            let handler_handle = conn.add_typed_handler(TrackerHandler::new(flag), Some(sender))?;
            conn.send_all_descriptions()?;
            for _ in 0..4 {
                let _ = conn.poll_endpoints(&mut cx)?;
            }
            conn.remove_handler(handler_handle)
                .expect("should be able to remove handler");
            Ok(())
        }
        futures::executor::block_on(function(&flag)).unwrap();

        assert!(flag.load(Ordering::SeqCst));
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker() {
        let flag = Arc::new(AtomicBool::new(false));
        async fn function(flag: &Arc<AtomicBool>) -> Result<()> {
            let mut cx = futures::task::Context::from_waker(futures::task::noop_waker_ref());
            let server = "127.0.0.1:3883".parse::<ServerInfo>()?;
            let conn = ConnectionIp::new_client(server, None, None)?;
            let sender = conn
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            let handler_handle = conn.add_typed_handler(TrackerHandler::new(flag), Some(sender))?;
            while conn.status() == ConnectionStatus::ClientConnecting {
                let _ = conn.poll_endpoints(&mut cx)?;
            }
            for _ in 0..4 {
                let _ = conn.poll_endpoints(&mut cx)?;
            }
            conn.remove_handler(handler_handle)
                .expect("should be able to remove handler");
            Ok(())
        }
        futures::executor::block_on(function(&flag)).unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }
    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker_manual() {
        let flag = Arc::new(AtomicBool::new(false));
        async fn function(flag: &Arc<AtomicBool>) -> Result<()> {
            let mut cx = futures::task::Context::from_waker(futures::task::noop_waker_ref());
            let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>()?;
            let conn = ConnectionIp::new_client(server, None, None)?;
            let tracker_message_id = conn
                .register_type(StaticMessageTypeName(b"vrpn_Tracker Pos_Quat"))
                .expect("should be able to register type");
            let sender = conn
                .register_sender(StaticSenderName(b"Tracker0"))
                .expect("should be able to register sender");
            conn.add_handler(
                TrackerHandler::new(flag),
                Some(tracker_message_id),
                Some(sender),
            )?;
            while conn.status() == ConnectionStatus::ClientConnecting {
                let _ = conn.poll_endpoints(&mut cx)?;
            }
            for _ in 0..4 {
                let _ = conn.poll_endpoints(&mut cx)?;
            }
            Ok(())
        }
        futures::executor::block_on(function(&flag)).unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }
}
