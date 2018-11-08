// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    base::{
        constants, Description, GenericMessage, InnerDescription, LocalId, LogFileNames, SenderId,
        SenderName, TypeId, TypeName,
    },
    buffer::message::unbuffer_typed_message_body,
    connect::ConnectError,
    connection::{
        translationtable::{
            Result as TranslationTableResult, TranslationTable, TranslationTableError,
        },
        typedispatcher::RegisterMapping,
        HandlerResult, MappingResult, TypeDispatcher,
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

fn append_error(
    old: TranslationTableResult<()>,
    new_err: TranslationTableError,
) -> TranslationTableResult<()> {
    match old {
        Err(old_e) => Err(old_e.append(new_err)),
        Ok(()) => Err(new_err),
    }
}

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
    use crate::connection::RegisterMapping::*;
    conn.type_dispatcher
        .set_system_handler(constants::SENDER_DESCRIPTION, move |&msg| {
            let desc = unbuffer_typed_message_body::<InnerDescription>(msg)?
                .into_typed_description::<SenderId>();

            let local_id = match conn.register_sender(SenderName(desc.name.as_ref()))? {
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
            remote_log_names: LogFileNames::from(remote_log_names),
            local_log_names: LogFileNames::from(local_log_names),
            endpoints: Arc::new(Mutex::new(Vec::new())),
            server_tcp: None,
        })
    }

    // fn endpoints_iter_mut(&mut self) -> Self::EndpointIteratorMut {
    //     self.endpoints.iter_mut()
    // }

    // fn endpoints_iter(&self) -> Self::EndpointIterator {
    //     self.endpoints.iter()
    // }

    // fn dispatcher(&self) -> &TypeDispatcher {
    //     &self.type_dispatcher
    // }

    // fn dispatcher_mut(&mut self) -> &mut TypeDispatcher {
    //     &mut self.type_dispatcher
    // }

    fn pack_sender_description(
        &mut self,
        name: SenderName,
        sender: SenderId,
    ) -> TranslationTableResult<()> {
        let sender = LocalId(sender);
        let mut my_result = Ok(());
        for endpoint in self.endpoints.lock().unwrap().iter_mut().flatten() {
            match endpoint.pack_sender_description(sender) {
                Ok(()) => (),
                Err(e) => {
                    my_result = append_error(my_result, e);
                }
            }
            endpoint.new_local_sender(name.clone(), sender);
        }
        my_result
    }

    fn pack_type_description(
        &mut self,
        name: TypeName,
        message_type: TypeId,
    ) -> TranslationTableResult<()> {
        let message_type = LocalId(message_type);
        let mut my_result = Ok(());
        for endpoint in self.endpoints.lock().unwrap().iter_mut().flatten() {
            match endpoint.pack_type_description(message_type) {
                Ok(()) => (),
                Err(e) => {
                    my_result = append_error(my_result, e);
                }
            }
            endpoint.new_local_type(name.clone(), message_type);
        }
        my_result
    }

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
    fn register_sender(&mut self, name: SenderName) -> MappingResult<RegisterMapping<SenderId>> {
        self.type_dispatcher.register_sender(name)
    }
    fn register_type(&mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>> {
        self.type_dispatcher.register_type(name)
    }
}
