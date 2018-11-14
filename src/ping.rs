// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use chrono::prelude::*;
use crate::{
    type_dispatcher::HandlerHandle, unbuffer::check_expected, BaseTypeSafeId, Buffer, BufferSize,
    BytesRequired, Connection, ConstantBufferSize, EmptyMessage, EmptyResult, Error, LocalId,
    Message, MessageHeader, MessageTypeIdentifier, Result, SenderId, SenderName, ServiceFlags,
    SomeId, StaticTypeName, TypeId, TypeSafeId, TypedBodylessHandler, TypedHandler,
    TypedMessageBody, Unbuffer,
};
use std::{
    fmt,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Ping;
const PING_MESSAGE: StaticTypeName = StaticTypeName(b"vrpn_Base ping_message");
impl Default for Ping {
    fn default() -> Ping {
        Ping
    }
}
impl EmptyMessage for Ping {}
impl TypedMessageBody for Ping {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(PING_MESSAGE);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Pong;
const PONG_MESSAGE: StaticTypeName = StaticTypeName(b"vrpn_Base pong_message");
impl Default for Pong {
    fn default() -> Pong {
        Pong
    }
}
impl EmptyMessage for Pong {}
impl TypedMessageBody for Pong {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(PONG_MESSAGE);
}

struct PongHandler<T: Connection> {
    inner: ClientInnerHolder<T>,
}

impl<T: Connection> fmt::Debug for PongHandler<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PongHandler").finish()
    }
}

impl<T: Connection> TypedBodylessHandler for PongHandler<T> {
    type Item = Pong;
    fn handle_typed_bodyless(&mut self, header: &MessageHeader) -> Result<()> {
        // TODO mark as alive/revived?
        let mut inner = self.inner.lock()?;
        inner.unanswered_ping = None;
        Ok(())
    }
}

pub struct Client<T: Connection + 'static> {
    inner: ClientInnerHolder<T>,
    ping_type: TypeId,
    sender: SenderId,
}

struct ClientInner<T: Connection> {
    connection: Arc<T>,
    unanswered_ping: Option<DateTime<Utc>>,
}

type ClientInnerHolder<T> = Arc<Mutex<ClientInner<T>>>;

impl<T: Connection + 'static> Client<T> {
    pub fn new(sender: LocalId<SenderId>, connection: Arc<T>) -> Result<Client<T>> {
        let ping_type = connection.register_type(PING_MESSAGE)?;
        let inner = ClientInner::new(Arc::clone(&connection));

        let _ = connection.add_typed_handler(
            Box::new(PongHandler {
                inner: Arc::clone(&inner),
            }),
            SomeId(sender.into_id()),
        )?;
        Ok(Client {
            inner,
            ping_type,
            sender: sender.into_id(),
        })
    }
    pub fn new_from_name(
        sender: impl Into<SenderName> + Clone,
        connection: Arc<T>,
    ) -> Result<Client<T>> {
        let sender_id = connection.register_sender(sender)?;
        Self::new(LocalId(sender_id), connection)
    }

    fn send_ping(&self) -> Result<()> {
        let msg = Message::new(None, self.ping_type, self.sender, Pong::default());
        let mut inner = self.inner.lock()?;
        inner
            .connection
            .pack_message(msg, ServiceFlags::RELIABLE.into())?;
        inner.unanswered_ping = Some(Utc::now());
        Ok(())
    }
}

impl<T: Connection> ClientInner<T> {
    fn new(connection: Arc<T>) -> Arc<Mutex<ClientInner<T>>> {
        Arc::new(Mutex::new(ClientInner {
            connection,
            unanswered_ping: None,
        }))
    }
}

#[derive(Debug)]
struct PingHandler<T: Connection> {
    connection: Arc<T>,
    pong_type: TypeId,
    sender: SenderId,
}

impl<T: Connection> TypedBodylessHandler for PingHandler<T> {
    type Item = Ping;
    fn handle_typed_bodyless(&mut self, _header: &MessageHeader) -> Result<()> {
        // TODO use sender from header?
        let msg = Message::new(None, self.pong_type, self.sender, Pong::default());
        self.connection
            .pack_message(msg, ServiceFlags::RELIABLE.into())?;
        Ok(())
    }
}

/// A struct that handles the ping/pong between client (sends ping)
/// and server (replies with pong)
#[derive(Debug)]
pub struct Server {
    handler: HandlerHandle,
}

impl Server {
    pub fn new<T: Connection + 'static>(
        sender: LocalId<SenderId>,
        connection: Arc<T>,
    ) -> Result<Server> {
        let connection_clone = Arc::clone(&connection);
        let pong_type = connection.register_type(PONG_MESSAGE)?;
        let handler = connection.add_typed_handler(
            Box::new(PingHandler {
                connection: connection_clone,
                pong_type,
                sender: sender.into_id(),
            }),
            SomeId(sender.into_id()),
        )?;
        Ok(Server { handler })
    }

    pub fn new_from_name<T: Connection + 'static>(
        sender: impl Into<SenderName> + Clone,
        connection: Arc<T>,
    ) -> Result<Server> {
        let sender_id = connection.register_sender(sender)?;
        Self::new(LocalId(sender_id), connection)
    }
}
