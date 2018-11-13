// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    type_dispatcher::{HandlerHandle, RegisterMapping},
    vrpn_tokio::endpoint_ip::EndpointIp,
    Error, Handler, IdToHandle, LogFileNames, Message, Result, SenderId, SenderName, SomeId,
    StaticSenderName, StaticTypeName, TypeDispatcher, TypeId, TypeName,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, TcpStream},
    prelude::*,
};

#[derive(Debug)]
pub struct ConnectionIp {
    endpoints: Arc<Mutex<Vec<Option<EndpointIp>>>>,
    pub(crate) type_dispatcher: Arc<Mutex<TypeDispatcher>>,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
    server_tcp: Option<TcpListener>,
}

impl ConnectionIp {
    /// Create a new ConnectionIp that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        addr: Option<SocketAddr>,
    ) -> Result<ConnectionIp> {
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let server_tcp = TcpListener::bind(&addr)?;
        Ok(ConnectionIp {
            endpoints: Arc::new(Mutex::new(Vec::new())),
            type_dispatcher: Arc::new(Mutex::new(TypeDispatcher::new())),
            remote_log_names: LogFileNames::new(),
            local_log_names: LogFileNames::from(local_log_names),
            server_tcp: Some(server_tcp),
        })
    }

    /// Create a new ConnectionIp that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> Result<ConnectionIp> {
        let mut endpoints: Vec<Option<EndpointIp>> = Vec::new();
        endpoints.push(Some(EndpointIp::new(reliable_channel)));
        Ok(ConnectionIp {
            endpoints: Arc::new(Mutex::new(endpoints)),
            type_dispatcher: Arc::new(Mutex::new(TypeDispatcher::new())),
            remote_log_names: LogFileNames::from(remote_log_names),
            local_log_names: LogFileNames::from(local_log_names),
            server_tcp: None,
        })
    }

    fn add_type(&self, name: impl Into<TypeName>) -> Result<TypeId> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.add_type(name)
    }

    fn add_sender(&self, name: impl Into<SenderName>) -> Result<SenderId> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.add_sender(name)
    }
    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: impl Into<TypeName>) -> Option<TypeId> {
        let dispatcher = self.type_dispatcher.lock().ok()?;
        dispatcher.get_type_id(name)
    }
    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: impl Into<SenderName>) -> Option<SenderId> {
        let dispatcher = self.type_dispatcher.lock().ok()?;
        dispatcher.get_sender_id(name)
    }

    pub fn register_type(&self, name: impl Into<TypeName>) -> Result<RegisterMapping<TypeId>> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.register_type(name)
    }

    pub fn register_sender(
        &self,
        name: impl Into<SenderName>,
    ) -> Result<RegisterMapping<SenderId>> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.register_sender(name)
    }

    pub fn add_handler(
        &self,
        handler: Box<dyn Handler>,
        message_type: IdToHandle<TypeId>,
        sender: IdToHandle<SenderId>,
    ) -> Result<HandlerHandle> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.add_handler(handler, message_type, sender)
    }

    pub fn remove_handler(&self, handler_handle: HandlerHandle) -> Result<()> {
        let mut dispatcher = self.type_dispatcher.lock()?;
        dispatcher.remove_handler(handler_handle)
    }

    fn poll_endpoints(&self) -> Poll<(), Error> {
        let endpoints = Arc::clone(&self.endpoints);
        let dispatcher = Arc::clone(&self.type_dispatcher);
        {
            let mut endpoints = endpoints.lock()?;
            let mut dispatcher = dispatcher.lock()?;
            let mut got_not_ready = false;
            for ep in endpoints.iter_mut().flatten() {
                match ep.poll_endpoint(&mut dispatcher)? {
                    Async::Ready(()) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tracker::*, TypedHandler};

    #[derive(Debug)]
    struct TrackerHandler {
        flag: Arc<Mutex<bool>>,
    }
    impl TypedHandler for TrackerHandler {
        type Item = PoseReport;
        fn handle_typed(&mut self, msg: &Message<PoseReport>) -> Result<()> {
            println!("{:?}", msg);
            let mut flag = self.flag.lock()?;
            *flag = true;
            Ok(())
        }
    }
    //#[ignore] // because it requires an external server to be running.
    #[test]
    fn tracker() {
        use crate::vrpn_tokio::connect_tcp;
        let addr = "127.0.0.1:3883".parse().unwrap();
        let flag = Arc::new(Mutex::new(false));

        connect_tcp(addr)
            .and_then(|stream| -> Result<()> {
                let conn = ConnectionIp::new_client(None, None, stream)?;
                let tracker_message_id = conn
                    .register_type(StaticTypeName(b"vrpn_Tracker Pos_Quat"))?
                    .get();
                let sender = conn.register_sender(StaticSenderName(b"Tracker0"))?.get();
                let handler_handle = conn.add_handler(
                    Box::new(TrackerHandler {
                        flag: Arc::clone(&flag),
                    }),
                    SomeId(tracker_message_id),
                    SomeId(sender),
                )?;
                for _ in 0..4 {
                    let _ = conn.poll_endpoints()?;
                }
                conn.remove_handler(handler_handle)?;
                Ok(())
            })
            .wait()
            .unwrap();
        assert!(*flag.lock().unwrap() == true);
    }
}
