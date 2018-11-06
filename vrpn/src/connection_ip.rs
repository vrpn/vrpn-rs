// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    connect::ConnectError,
    endpoint_ip::{EndpointIP, MessageFramed, MessageFramedUdp},
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use vrpn_base::types::{HandlerParams, SenderId, SenderName, TypeId, TypeName};
use vrpn_connection::{
    make_log_names, make_none_log_names, Connection, HandlerResult, LogFileNames, MappingResult,
    TypeDispatcher,
};

pub struct ConnectionIP {
    type_dispatcher: TypeDispatcher<'static>,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
    endpoints: Vec<Option<EndpointIP>>,
    server_tcp: Option<TcpListener>,
}

impl ConnectionIP {
    /// Common initialization
    fn init(&mut self) -> HandlerResult<()> {
        let handle_udp_message = |params: HandlerParams| -> HandlerResult<()> { Ok(()) };
        /*
        self.type_dispatcher
            .set_system_handler(constants::UDP_DESCRIPTION, handle_udp_message)
            */
        Ok(())
    }

    /// Create a new ConnectionIP that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        addr: Option<SocketAddr>,
    ) -> Result<ConnectionIP, ConnectError> {
        let disp = TypeDispatcher::new()?;
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let listener = TcpListener::bind(&addr)?;
        let mut ret = ConnectionIP {
            type_dispatcher: disp,
            remote_log_names: make_none_log_names(),
            local_log_names: make_log_names(local_log_names),
            endpoints: Vec::new(),
            server_tcp: Some(listener),
        };
        ret.init()?;
        Ok(ret)
    }

    /// Create a new ConnectionIP that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> HandlerResult<ConnectionIP> {
        let disp = TypeDispatcher::new()?;
        let mut ret = ConnectionIP {
            type_dispatcher: disp,
            remote_log_names: make_log_names(remote_log_names),
            local_log_names: make_log_names(local_log_names),
            endpoints: Vec::new(),
            server_tcp: None,
        };
        // Create our single endpoint
        ret.endpoints.push(Some(EndpointIP::new(reliable_channel)));
        ret.init()?;
        Ok(ret)
    }
}

impl<'a> Connection<'a> for ConnectionIP {
    type EndpointItem = EndpointIP;
    type EndpointIteratorMut = std::slice::IterMut<'a, Option<EndpointIP>>;
    type EndpointIterator = std::slice::Iter<'a, Option<EndpointIP>>;

    fn endpoints_iter_mut(&'a mut self) -> Self::EndpointIteratorMut {
        self.endpoints.iter_mut()
    }

    fn endpoints_iter(&'a self) -> Self::EndpointIterator {
        self.endpoints.iter()
    }

    // fn dispatcher(&self) -> &TypeDispatcher {
    //     &self.type_dispatcher
    // }

    // fn dispatcher_mut(&'a mut self) -> &'a mut TypeDispatcher {
    //     &mut self.type_dispatcher
    // }

    fn add_type(&mut self, name: TypeName) -> MappingResult<TypeId> {
        self.type_dispatcher.add_type(name)
    }

    fn add_sender(&mut self, name: SenderName) -> MappingResult<SenderId> {
        self.type_dispatcher.add_sender(name)
    }
    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: &TypeName) -> Option<TypeId> {
        self.type_dispatcher.get_type_id(name)
    }
    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: &SenderName) -> Option<SenderId> {
        self.type_dispatcher.get_sender_id(name)
    }
    // fn register_sender(&'a mut self, name: SenderName) -> MappingResult<RegisterMapping<SenderId>> ;
    // fn register_type(&'a mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>>;
}
