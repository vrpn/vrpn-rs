// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    base::{
        constants, Description, GenericMessage, InnerDescription, LocalId, LogFileNames, RemoteId,
        SenderId, SenderName, TypeId, TypeName,
    },
    buffer::message::unbuffer_typed_message_body,
    connection::{
        append_error, typedispatcher::RegisterMapping, Endpoint, Error as ConnectionError,
        Result as ConnectionResult, SystemHandler, TypeDispatcher,
    },
    endpoint_ip::{EndpointIp, MessageFramed, MessageFramedUdp},
    error::Error,
    prelude::*,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::DerefMut,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub(crate) struct ConnectionIpInner {
    pub(crate) type_dispatcher: TypeDispatcher,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
    endpoints: Vec<Option<EndpointIp>>,
    server_tcp: Option<TcpListener>,
}

impl ConnectionIpInner {
    fn pack_sender_description<T: Into<SenderName>>(
        &mut self,
        name: T,
        sender: SenderId,
    ) -> ConnectionResult<()> {
        let name: SenderName = name.into();
        let sender = LocalId(sender);
        let mut my_result = Ok(());
        for endpoint in self.endpoints.iter_mut().flatten() {
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

    fn pack_type_description<T: Into<TypeName>>(
        &mut self,
        name: T,
        message_type: TypeId,
    ) -> ConnectionResult<()> {
        let name: TypeName = name.into();
        let message_type = LocalId(message_type);
        let mut my_result = Ok(());
        for endpoint in self.endpoints.iter_mut().flatten() {
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
}

pub(crate) type ArcConnectionIpInner = Arc<Mutex<ConnectionIpInner>>;

pub(crate) fn inner_lock_mut<'a, E>(
    inner: &'a mut ArcConnectionIpInner,
) -> std::result::Result<MutexGuard<'a, ConnectionIpInner>, E>
where
    E: std::error::Error + From<String>,
{
    inner.lock().map_err(|e| E::from(e.to_string()))
}

pub(crate) fn inner_lock<'a, E>(
    inner: &'a ArcConnectionIpInner,
) -> std::result::Result<MutexGuard<'a, ConnectionIpInner>, E>
where
    E: std::error::Error + From<String>,
{
    inner.lock().map_err(|e| E::from(e.to_string()))
}

pub(crate) fn inner_lock_option<'a>(
    inner: &'a ArcConnectionIpInner,
) -> Option<MutexGuard<'a, ConnectionIpInner>> {
    match inner.lock() {
        Ok(guard) => Some(guard),
        Err(_) => None,
    }
}

#[derive(Debug)]
pub struct ConnectionIp {
    inner: ArcConnectionIpInner,
}

impl ConnectionIp {
    /// Create a new ConnectionIp that is a server.
    pub fn new_server(
        local_log_names: Option<LogFileNames>,
        addr: Option<SocketAddr>,
    ) -> Result<ConnectionIp, Error> {
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let listener = TcpListener::bind(&addr)?;
        let mut inner = Self::new_inner(None, local_log_names)?;
        {
            let mut inner = inner_lock_mut::<Error>(&mut inner)?;
            inner.server_tcp = Some(listener);
        }
        Self::new_impl(inner).map_err(|e| e.into())
    }

    /// Create a new ConnectionIp that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> ConnectionResult<ConnectionIp> {
        let mut inner = Self::new_inner(remote_log_names, local_log_names)?;
        {
            let mut inner = inner_lock_mut::<ConnectionError>(&mut inner)?;
            inner
                .endpoints
                .push(Some(EndpointIp::new(reliable_channel)));
        }
        Self::new_impl(inner)
    }

    fn new_inner(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> ConnectionResult<ArcConnectionIpInner> {
        Ok(Arc::new(Mutex::new(ConnectionIpInner {
            type_dispatcher: TypeDispatcher::new(),
            remote_log_names: LogFileNames::from(remote_log_names),
            local_log_names: LogFileNames::from(local_log_names),
            endpoints: Vec::new(),
            server_tcp: None,
        })))
    }
    /// Common new implementation
    fn new_impl(inner: ArcConnectionIpInner) -> ConnectionResult<ConnectionIp> {
        {
            let conn = Arc::clone(&inner);
            // {
            //     let mut inner = Arc::clone(&inner);
            //     let mut inner = inner_lock_mut::<ConnectionError>(&mut inner)?;
            //     /*
            //     self.type_dispatcher
            //         .set_system_handler(constants::UDP_DESCRIPTION, handle_udp_message)
            //         */
            //     inner
            //         .type_dispatcher
            //         .set_system_handler(SenderDescriptionHandler::new(&conn))?;
            //     inner
            //         .type_dispatcher
            //         .set_system_handler(TypeDescriptionHandler::new(&conn))?;
            //     // conn.type_dispatcher
            //     //     .set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
            // }
        }
        Ok(ConnectionIp { inner })
    }

    fn pack_sender_description(
        &mut self,
        name: impl Into<SenderName>,
        sender: SenderId,
    ) -> ConnectionResult<()> {
        self.inner_lock_mut()?.pack_sender_description(name, sender)
    }

    fn pack_type_description(
        &mut self,
        name: impl Into<TypeName>,
        message_type: TypeId,
    ) -> ConnectionResult<()> {
        self.inner_lock_mut()?
            .pack_type_description(name, message_type)
    }

    fn add_type(&mut self, name: impl Into<TypeName>) -> ConnectionResult<TypeId> {
        self.inner_lock_mut()?.type_dispatcher.add_type(name)
    }

    fn add_sender(&mut self, name: impl Into<SenderName>) -> ConnectionResult<SenderId> {
        self.inner_lock_mut()?.type_dispatcher.add_sender(name)
    }
    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: impl Into<TypeName>) -> Option<TypeId> {
        self.inner_lock_option()?.type_dispatcher.get_type_id(name)
    }
    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: impl Into<SenderName>) -> Option<SenderId> {
        self.inner_lock_option()?
            .type_dispatcher
            .get_sender_id(name)
    }

    fn register_type(
        &mut self,
        name: impl Into<TypeName>,
    ) -> ConnectionResult<RegisterMapping<TypeId>> {
        self.inner_lock_mut()?.type_dispatcher.register_type(name)
    }

    fn register_sender(
        &mut self,
        name: impl Into<SenderName>,
    ) -> ConnectionResult<RegisterMapping<SenderId>> {
        self.inner_lock_mut()?.type_dispatcher.register_sender(name)
    }

    fn inner_lock_mut(&mut self) -> ConnectionResult<MutexGuard<ConnectionIpInner>> {
        inner_lock_mut(&mut self.inner)
    }

    fn inner_lock<E>(&self) -> std::result::Result<MutexGuard<ConnectionIpInner>, E>
    where
        E: std::error::Error + From<String>,
    {
        inner_lock(&self.inner)
    }

    fn inner_lock_option(&self) -> Option<MutexGuard<ConnectionIpInner>> {
        inner_lock_option(&self.inner)
    }
}
// #[derive(Debug)]
// struct SenderDescriptionHandler {
//     conn: ArcConnectionIpInner,
// }

// impl SenderDescriptionHandler {
//     fn new(conn: &ArcConnectionIpInner) -> Box<dyn SystemHandler> {
//         Box::new(SenderDescriptionHandler {
//             conn: Arc::clone(conn),
//         })
//     }
// }
// impl SystemHandler for SenderDescriptionHandler {
//     fn message_type(&self) -> TypeId {
//         constants::SENDER_DESCRIPTION
//     }
//     fn handle(
//         &mut self,
//         msg: &GenericMessage,
//         endpoint: &mut dyn Endpoint,
//     ) -> ConnectionResult<()> {
//         let msg = msg.clone();
//         let mut conn = inner_lock_mut::<ConnectionError>(&mut self.conn)?;
//         let desc = unbuffer_typed_message_body::<InnerDescription>(msg)?
//             .into_typed_description::<SenderId>();
//         let name = desc.name;
//         let local_id = conn
//             .type_dispatcher
//             .register_sender(SenderName(name.clone()))?
//             .get();
//         // for ep in conn.endpoints.iter_mut().flatten() {
//         //     let _ = ep.sender_table_mut().add_remote_entry(
//         //         name.clone(),
//         //         RemoteId(desc.which),
//         //         LocalId(local_id),
//         //     )?;
//         // }
//         Ok(())
//     }
// }

// #[derive(Debug)]
// struct TypeDescriptionHandler {
//     conn: ArcConnectionIpInner,
// }

// impl TypeDescriptionHandler {
//     fn new(conn: &ArcConnectionIpInner) -> Box<dyn SystemHandler> {
//         Box::new(TypeDescriptionHandler {
//             conn: Arc::clone(conn),
//         })
//     }
// }

// impl SystemHandler for TypeDescriptionHandler {
//     fn message_type(&self) -> TypeId {
//         constants::TYPE_DESCRIPTION
//     }
//     fn handle(
//         &mut self,
//         msg: &GenericMessage,
//         endpoint: &mut dyn Endpoint,
//     ) -> ConnectionResult<()> {
//         let msg = msg.clone();
//         let mut conn = inner_lock_mut::<ConnectionError>(&mut self.conn)?;
//         let desc = unbuffer_typed_message_body::<InnerDescription>(msg)?
//             .into_typed_description::<TypeId>();
//         let name = desc.name;
//         let local_id = conn
//             .type_dispatcher
//             .register_type(TypeName(name.clone()))?
//             .get();
//         for ep in conn.endpoints.iter_mut().flatten() {
//             let _ = ep.type_table_mut().add_remote_entry(
//                 name.clone(),
//                 RemoteId(desc.which),
//                 LocalId(local_id),
//             )?;
//         }
//         Ok(())
//     }
// }

// #[derive(Debug)]
// struct UdpDescriptionHandler {
//     conn: ArcConnectionIpInner,
// }

// impl UdpDescriptionHandler {
//     fn new(conn: &ArcConnectionIpInner) -> Box<dyn SystemHandler> {
//         Box::new(UdpDescriptionHandler {
//             conn: Arc::clone(conn),
//         })
//     }
// }

// impl SystemHandler for UdpDescriptionHandler {
//     fn message_type(&self) -> TypeId {
//         constants::UDP_DESCRIPTION
//     }
//     fn handle(
//         &mut self,
//         msg: &GenericMessage,
//         endpoint: &mut dyn Endpoint,
//     ) -> ConnectionResult<()> {
//         let msg = msg.clone();
//         let mut conn = inner_lock_mut::<ConnectionError>(&mut self.conn)?;
//         let ip: Vec<u8> = msg
//             .body
//             .inner
//             .iter()
//             .take_while(|b| **b != 0)
//             .cloned()
//             .collect();
//         let port = msg.header.sender.get();

//         let desc = unbuffer_typed_message_body::<InnerDescription>(msg)?
//             .into_typed_description::<TypeId>();
//         let name = desc.name;
//         let local_id = conn
//             .type_dispatcher
//             .register_type(TypeName(name.clone()))?
//             .get();
//         for ep in conn.endpoints.iter_mut().flatten() {
//             let _ = ep.type_table_mut().add_remote_entry(
//                 name.clone(),
//                 RemoteId(desc.which),
//                 LocalId(local_id),
//             )?;
//         }
//         Ok(())
//     }
// }
