// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    base::{Error, LogFileNames, Result, SenderId, SenderName, TypeId, TypeName},
    connection::{typedispatcher::RegisterMapping, TypeDispatcher},
    endpoint_ip::EndpointIp,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
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

impl ConnectionIpInner {}

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
    ) -> Result<ConnectionIp> {
        let addr =
            addr.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0));
        let listener = TcpListener::bind(&addr)?;
        let mut inner = Self::new_inner(None, local_log_names)?;
        {
            let mut inner = inner_lock_mut::<Error>(&mut inner)?;
            inner.server_tcp = Some(listener);
        }
        Self::new_impl(inner)
    }

    /// Create a new ConnectionIp that is a client.
    pub fn new_client(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
        reliable_channel: TcpStream,
        // low_latency_channel: Option<MessageFramedUdp>,
    ) -> Result<ConnectionIp> {
        let mut inner = Self::new_inner(remote_log_names, local_log_names)?;
        {
            let mut inner = inner_lock_mut::<Error>(&mut inner)?;
            inner
                .endpoints
                .push(Some(EndpointIp::new(reliable_channel)));
        }
        Self::new_impl(inner)
    }

    fn new_inner(
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> Result<ArcConnectionIpInner> {
        Ok(Arc::new(Mutex::new(ConnectionIpInner {
            type_dispatcher: TypeDispatcher::new(),
            remote_log_names: LogFileNames::from(remote_log_names),
            local_log_names: LogFileNames::from(local_log_names),
            endpoints: Vec::new(),
            server_tcp: None,
        })))
    }
    /// Common new implementation
    fn new_impl(inner: ArcConnectionIpInner) -> Result<ConnectionIp> {
        Ok(ConnectionIp { inner })
    }

    fn add_type(&mut self, name: impl Into<TypeName>) -> Result<TypeId> {
        self.inner_lock_mut()?.type_dispatcher.add_type(name)
    }

    fn add_sender(&mut self, name: impl Into<SenderName>) -> Result<SenderId> {
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

    fn register_type(&mut self, name: impl Into<TypeName>) -> Result<RegisterMapping<TypeId>> {
        self.inner_lock_mut()?.type_dispatcher.register_type(name)
    }

    fn register_sender(
        &mut self,
        name: impl Into<SenderName>,
    ) -> Result<RegisterMapping<SenderId>> {
        self.inner_lock_mut()?.type_dispatcher.register_sender(name)
    }

    fn inner_lock_mut(&mut self) -> Result<MutexGuard<ConnectionIpInner>> {
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
