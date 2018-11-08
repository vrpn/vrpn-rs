// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    base::{
        constants,
        message::{Description, GenericMessage},
        types::{SenderId, SenderName, TypeId, TypeName},
    },
    buffer::message::unbuffer_typed_message_body,
    connect::ConnectError,
    connection::typedispatcher::RegisterMapping,
    connection::{
        make_log_names, make_none_log_names, Connection, HandlerResult, LogFileNames,
        MappingResult, TypeDispatcher,
    },
    endpoint_ip::{EndpointIP, MessageFramed, MessageFramedUdp},
    prelude::*,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::DerefMut,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};

pub struct ConnectionIP {
    type_dispatcher: Arc<TypeDispatcher<'static>>,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
    endpoints: Arc<Mutex<Vec<Option<EndpointIP>>>>,
    server_tcp: Option<TcpListener>,
}

/// Common initialization
fn init(conn: &mut Arc<ConnectionIP>) -> HandlerResult<()> {
    let conn = Arc::clone(conn);
    let handle_udp_message = |params: GenericMessage| -> HandlerResult<()> { Ok(()) };
    /*
    self.type_dispatcher
        .set_system_handler(constants::UDP_DESCRIPTION, handle_udp_message)
        */

    conn.type_dispatcher
        .set_system_handler(constants::SENDER_DESCRIPTION, move |&msg| {
            let desc = unbuffer_typed_message_body::<InnerDescription>(msg)?
                .into_typed_description::<SenderId>();

            let local_id = match conn.register_sender(desc.name.clone()) {
                Found(v) => v,
                NewMapping(v) => v,
            };
            let mut endpoints = conn.endpoints().lock().unwrap();
            for ep in endpoints.iter_mut().flatten() {
                let _ = ep.sender_table_mut().add_remote_entry(
                    desc.name.clone(),
                    RemoteId(desc.which),
                    LocalId(local_id),
                )?;
            }
            Ok(())
        });
    // conn.type_dispatcher
    //     .set_system_handler(constants::TYPE_DESCRIPTION, handle_type_message);
    // conn.type_dispatcher
    //     .set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
    Ok(())
}

impl ConnectionIP {
    fn endpoints(&self) -> Arc<Mutex<Vec<Option<EndpointIP>>>> {
        Arc::clone(&self.endpoints)
    }

    /// Create a new ConnectionIP that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        addr: Option<SocketAddr>,
    ) -> Result<Arc<ConnectionIP>, ConnectError> {
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let listener = TcpListener::bind(&addr)?;
        let mut ret = Arc::new(ConnectionIP {
            server_tcp: Some(listener),
            ..Self::inner_create(None, local_log_names)?
        });
        init(&mut ret)?;
        Ok(ret)
    }

    /// Create a new ConnectionIP that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> HandlerResult<Arc<ConnectionIP>> {
        let mut ret = Arc::new(Self::inner_create(remote_log_names, local_log_names)?);
        // Create our single endpoint
        ret.endpoints
            .lock()
            .unwrap()
            .push(Some(EndpointIP::new(reliable_channel)));
        init(&mut ret)?;
        Ok(ret)
    }

    fn inner_create(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> HandlerResult<ConnectionIP> {
        let disp = TypeDispatcher::new()?;
        Ok(ConnectionIP {
            type_dispatcher: Arc::new(disp),
            remote_log_names: make_log_names(remote_log_names),
            local_log_names: make_log_names(local_log_names),
            endpoints: Arc::new(Mutex::new(Vec::new())),
            server_tcp: None,
        })
    }
}

impl<'a> Connection<'a> for ConnectionIP {
    type EndpointItem = EndpointIP;
    type EndpointIteratorMut = std::slice::IterMut<'a, Option<EndpointIP>>;
    type EndpointIterator = std::slice::Iter<'a, Option<EndpointIP>>;

    // fn endpoints_iter_mut(&'a mut self) -> Self::EndpointIteratorMut {
    //     self.endpoints.iter_mut()
    // }

    // fn endpoints_iter(&'a self) -> Self::EndpointIterator {
    //     self.endpoints.iter()
    // }

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
    fn register_sender(&'a mut self, name: SenderName) -> MappingResult<RegisterMapping<SenderId>> {
        self.type_dispatcher.register_sender(name)
    }
    fn register_type(&'a mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>> {
        self.type_dispatcher.register_type(name)
    }
}
