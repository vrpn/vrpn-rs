// Copyright 2018-2022, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    connection::*,
    data_types::id_types::Id,
    data_types::log::LogFileNames,
    vrpn_tokio::{
        // codec::FramedMessageCodec,
        connect::{incoming_handshake, ConnectionIpInfo},
        endpoint_ip::EndpointIp,
    },
    Result, ServerInfo, VrpnError,
};
use futures::{ready, Future, FutureExt, Stream};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, Weak},
    task::Poll,
};
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct ConnectionIp {
    core: ConnectionCore<EndpointIp>,
    // server_tcp: Option<Mutex<TcpListener>>,
    server_acceptor: Arc<Mutex<Option<ConnectionIpAcceptor>>>,
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
            server_acceptor: Arc::new(Mutex::new(None)),
            // server_tcp: Some(Mutex::new(server_tcp)),
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
            server_acceptor: Arc::new(Mutex::new(None)),
            client_info: Mutex::new(ConnectionIpInfo::new_client(server)?),
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
            let num_endpoints = endpoints.len();
            if let Some(results) = ready!(client_info.poll(num_endpoints))? {
                todo!();
                // OK, we finished a connection setup.
                endpoints.push(Some(EndpointIp::new(
                    results.tcp.unwrap(),
                    results.udp, // .map(|sock| UdpFramed::new(sock, FramedMessageCodec)),
                )));
            };
        }

        let mut acceptor = self.server_acceptor.lock()?;
        match &mut (*acceptor) {
            Some(a) => loop {
                let poll_result = a.poll()?;
                match poll_result {
                    Poll::Pending => break,
                    Poll::Ready(Some(_)) => (),
                    Poll::Ready(None) => return Poll::Ready(Ok(None)),
                }
            },
            None => (),
        }
        let endpoints = self.endpoints();
        let dispatcher = self.dispatcher();
        {
            let mut endpoints = endpoints.lock()?;
            let mut dispatcher = dispatcher.lock()?;
            let mut got_not_ready = false;
            // Go through and poll each endpoint, "taking" the ones that are closed.
            for ep in endpoints.iter_mut() {
                let ready = match ep {
                    Some(endpoint) => match endpoint.poll_endpoint(&mut dispatcher) {
                        Poll::Ready(Err(e)) => {
                            println!("Got endpoint error: {:?}", e);
                            true
                        }
                        Poll::Ready(_) => true,
                        Poll::Pending => false,
                    },
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

#[derive(Debug)]
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
        self.connection.poll_endpoints(cx)
    }
}

#[derive(Debug)]
pub struct ConnectionIpAcceptor {
    connection: Weak<ConnectionIp>,
    // server_tcp: Mutex<Incoming<'a>>,
}
impl ConnectionIpAcceptor {
    pub fn new(
        connection: Weak<ConnectionIp>,
        addr: Option<SocketAddr>,
    ) -> Result<ConnectionIpAcceptor> {
        let addr = addr.unwrap_or_else(|| {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), DEFAULT_PORT)
        });
        // let server_tcp = Mutex::new(TcpListener::bind(&addr)?.incoming());
        Ok(ConnectionIpAcceptor {
            connection,
            // server_tcp,
        })
    }
}
impl Stream for ConnectionIpAcceptor {
    type Item = Result<()>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut incoming = self.server_tcp.lock()?;
        loop {
            let connection = match self.connection.upgrade() {
                Some(c) => c,
                None => return Ok(Poll::Ready(None)),
            };
            let socket = match ready!(incoming.poll()) {
                Some(s) => s,
                None => return Ok(Poll::Ready(None)),
            };
            // OK, we got a new one.
            let endpoints = connection.endpoints();
            tokio::spawn(
                incoming_handshake(socket)
                    .and_then(move |stream| {
                        if let Ok(peer) = stream.peer_addr() {
                            eprintln!("Got connection from {:?}", peer);
                        } else {
                            eprintln!("Got connection from some peer we couldn't identify");
                        }
                        if let Ok(mut epoints) = endpoints.lock() {
                            // TODO set up udp
                            epoints.push(Some(EndpointIp::new(stream, None)));
                        }
                        Ok(())
                    })
                    .map_err(|e| {
                        eprintln!("err: {:?}", e);
                    }),
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_types::{Message, StaticMessageTypeName, StaticSenderName, TypedMessage},
        handler::{HandlerCode, TypedHandler},
        tracker::*,
        Id,
    };
    use futures::future::IntoFuture;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct TrackerHandler {
        flag: Arc<Mutex<bool>>,
    }
    impl TypedHandler for TrackerHandler {
        type Item = PoseReport;
        fn handle_typed(&mut self, msg: &TypedMessage<PoseReport>) -> Result<HandlerCode> {
            println!("{:?}", msg);
            let mut flag = self.flag.lock()?;
            *flag = true;
            Ok(HandlerCode::ContinueProcessing)
        }
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker_tcp() {
        let flag = Arc::new(Mutex::new(false));
        let _ = "tcp://127.0.0.1:3883"
            .parse::<ServerInfo>()
            .into_future()
            .and_then(|server| {
                let conn = ConnectionIp::new_client(server, None, None)?;
                let sender = conn
                    .register_sender(StaticSenderName(b"Tracker0"))
                    .expect("should be able to register sender");
                let handler_handle = conn.add_typed_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    Some(sender),
                )?;
                conn.pack_all_descriptions()?;
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                conn.remove_handler(handler_handle)
                    .expect("should be able to remove handler");
                Ok(Poll::Ready(()))
            })
            .wait()
            .unwrap();

        assert!(*flag.lock().unwrap());
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker() {
        let flag = Arc::new(Mutex::new(false));

        let _ = "127.0.0.1:3883"
            .parse::<ServerInfo>()
            .into_future()
            .and_then(|server| {
                let conn = ConnectionIp::new_client(server, None, None)?;
                let sender = conn
                    .register_sender(StaticSenderName(b"Tracker0"))
                    .expect("should be able to register sender");
                let handler_handle = conn.add_typed_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    Some(sender),
                )?;
                while conn.status() == ConnectionStatus::ClientConnecting {
                    let _ = conn.poll_endpoints()?;
                }
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                conn.remove_handler(handler_handle)
                    .expect("should be able to remove handler");
                Ok(Poll::Ready(()))
            })
            .wait()
            .unwrap();
        assert!(*flag.lock().unwrap());
    }
    #[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker_manual() {
        let flag = Arc::new(Mutex::new(false));

        let _server = "tcp://127.0.0.1:3883"
            .parse::<ServerInfo>()
            .into_future()
            .and_then(|server| {
                let conn = ConnectionIp::new_client(server, None, None)?;
                let tracker_message_id = conn
                    .register_type(StaticMessageTypeName(b"vrpn_Tracker Pos_Quat"))
                    .expect("should be able to register type");
                let sender = conn
                    .register_sender(StaticSenderName(b"Tracker0"))
                    .expect("should be able to register sender");
                conn.add_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    Some(tracker_message_id),
                    Some(sender),
                )?;
                while conn.status() == ConnectionStatus::ClientConnecting {
                    let _ = conn.poll_endpoints()?;
                }
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                Ok(Poll::Ready(()))
            })
            .wait()
            .unwrap();
        assert!(*flag.lock().unwrap());
    }
}
