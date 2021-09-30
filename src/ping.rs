// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Types and functions for the periodic ping/pong messages in the VRPN protocol.

use crate::{
    buffer_unbuffer::EmptyMessage,
    data_types::{
        id_types::*, name_types::NameIntoBytes, ClassOfService, MessageHeader, MessageTypeId,
        MessageTypeIdentifier, SenderName, StaticMessageTypeName, TypedMessage, TypedMessageBody,
    },
    handler::{HandlerCode, HandlerHandle, TypedBodylessHandler},
    Connection, VrpnError,
};
use chrono::{prelude::*, Duration};
use std::{
    fmt,
    sync::{Arc, Mutex, Weak},
};

/// Periodic "Ping" message.
///
/// Has no body.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Ping;
const PING_MESSAGE: StaticMessageTypeName = StaticMessageTypeName(b"vrpn_Base ping_message");
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

/// "Pong" message, sent in reply to a `Ping` message
///
/// Has no body.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Pong;
const PONG_MESSAGE: StaticMessageTypeName = StaticMessageTypeName(b"vrpn_Base pong_message");
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

struct PongHandler {
    inner: Weak<Mutex<ClientInner>>,
}

impl fmt::Debug for PongHandler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PongHandler").finish()
    }
}

impl TypedBodylessHandler for PongHandler {
    type Item = Pong;
    fn handle_typed_bodyless(&mut self, _header: &MessageHeader) -> Result<HandlerCode, VrpnError> {
        match self.inner.upgrade() {
            Some(inner) => {
                let mut inner = inner.lock()?;
                inner.unanswered_ping = None;
                inner.last_warning = None;
                if inner.flatlined {
                    eprintln!("Remote host started responding again");
                    inner.flatlined = false;
                }
                Ok(HandlerCode::ContinueProcessing)
            }

            // If we get here, then the inner has gone away
            None => Ok(HandlerCode::RemoveThisHandler),
        }
    }
}

pub struct Client<T: Connection + 'static> {
    connection: Arc<T>,
    inner: Arc<Mutex<ClientInner>>,
    ping_type: LocalId<MessageTypeId>,
    sender: LocalId<SenderId>,
}

struct ClientInner {
    /// The time of the first unanswered ping.
    unanswered_ping: Option<DateTime<Utc>>,
    /// The time of the last warning message and unanswered ping.
    last_warning: Option<DateTime<Utc>>,
    /// whether the server seems disconnected or unresponsive
    flatlined: bool,
}

impl ClientInner {
    fn new() -> Arc<Mutex<ClientInner>> {
        Arc::new(Mutex::new(ClientInner {
            unanswered_ping: None,
            last_warning: None,
            flatlined: false,
        }))
    }
}
impl<T: Connection + 'static> Client<T> {
    pub fn new(sender: LocalId<SenderId>, connection: Arc<T>) -> Result<Client<T>, VrpnError> {
        let ping_type = connection.register_type(PING_MESSAGE)?;
        let inner = ClientInner::new();

        let _ = connection.add_typed_handler(
            Box::new(PongHandler {
                inner: Arc::downgrade(&inner),
            }),
            Some(sender),
        )?;
        let client = Client {
            connection,
            inner,
            ping_type,
            sender,
        };
        client.initiate_ping_cycle()?;
        Ok(client)
    }
    pub fn new_from_name(
        sender: impl Into<SenderName> + NameIntoBytes + Clone,
        connection: Arc<T>,
    ) -> Result<Client<T>, VrpnError> {
        let sender_id = connection.register_sender(sender)?;
        Self::new(sender_id, connection)
    }

    pub fn initiate_ping_cycle(&self) -> Result<(), VrpnError> {
        {
            let mut inner = self.inner.lock()?;
            inner.unanswered_ping = Some(Utc::now());
        }
        self.send_ping()
    }

    /// Checks to see if we're due for another ping.
    ///
    /// Returns the duration since the first unanswered ping,
    /// or None if there are no unanswered pings.
    pub fn check_ping_cycle(&self) -> Result<Option<Duration>, VrpnError> {
        let mut inner = self.inner.lock()?;
        if let (Some(unanswered), Some(last_warning)) =
            (inner.unanswered_ping, &mut inner.last_warning)
        {
            let now = Utc::now();
            let radio_silence = now.signed_duration_since(unanswered);
            if now.signed_duration_since(*last_warning) > Duration::seconds(1) {
                *last_warning = now;
                if radio_silence > Duration::seconds(10) {
                    inner.flatlined = true;
                }
                self.send_ping()?;
            }
            Ok(Some(radio_silence))
        } else {
            Ok(None)
        }
    }

    fn send_ping(&self) -> Result<(), VrpnError> {
        let msg = TypedMessage::new(None, self.ping_type, self.sender, Pong::default());
        self.connection
            .pack_message(msg, ClassOfService::RELIABLE)?;
        Ok(())
    }
}

#[derive(Debug)]
struct PingHandler<T: Connection> {
    connection: Weak<T>,
    pong_type: LocalId<MessageTypeId>,
    sender: LocalId<SenderId>,
}

impl<T: Connection + Send> TypedBodylessHandler for PingHandler<T> {
    type Item = Ping;
    fn handle_typed_bodyless(&mut self, _header: &MessageHeader) -> Result<HandlerCode, VrpnError> {
        // TODO use sender from header?
        match self.connection.upgrade() {
            Some(connection) => {
                let msg = TypedMessage::new(None, self.pong_type, self.sender, Pong::default());
                connection.pack_message(msg, ClassOfService::RELIABLE)?;
                Ok(HandlerCode::ContinueProcessing)
            }
            None => Ok(HandlerCode::RemoveThisHandler),
        }
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
    ) -> Result<Server, VrpnError> {
        let pong_type = connection.register_type(PONG_MESSAGE)?;
        let handler = connection.add_typed_handler(
            Box::new(PingHandler {
                connection: Arc::downgrade(&connection),
                pong_type,
                sender,
            }),
            Some(sender),
        )?;
        Ok(Server { handler })
    }

    pub fn new_from_name<T: Connection + 'static>(
        sender: impl Into<SenderName> + NameIntoBytes + Clone,
        connection: Arc<T>,
    ) -> Result<Server, VrpnError> {
        let sender_id = connection.register_sender(sender)?;
        Self::new(sender_id, connection)
    }
}
