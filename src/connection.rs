// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::{
    convert::TryFrom,
    sync::{Arc, Mutex},
};

use crate::{
    buffer_unbuffer::BufferTo,
    data_types::{
        descriptions::{IdWithDescription, InnerDescription},
        id_types::*,
        ClassOfService, GenericMessage, LogFileNames, MessageTypeId, MessageTypeIdentifier,
        MessageTypeName, SenderName, TimeVal, TypedMessage, TypedMessageBody,
    },
    type_dispatcher::HandlerHandle,
    Endpoint, EndpointGeneric, Handler, MatchingTable, RegisterMapping, Result, TranslationTables,
    TypeDispatcher, TypedHandler,
};

pub type EndpointVec<EP> = Vec<Option<EP>>;
pub type SharedEndpointVec<EP> = Arc<Mutex<EndpointVec<EP>>>;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConnectionStatus {
    /// This is a client connection that is attempting to connect.
    ClientConnecting,
    /// This is a client connection that is successfully connected.
    ClientConnected,
    /// This is a server connection, the number of connected endpoints is provided
    Server(usize),
}

pub trait Connection: Send + Sync {
    type SpecificEndpoint: Endpoint + EndpointGeneric;

    /// Access the ConnectionCore nested struct.
    ///
    /// This is the main required method for this trait.
    fn connection_core(&self) -> &ConnectionCore<Self::SpecificEndpoint>;

    /// Get the status of this connection
    fn status(&self) -> ConnectionStatus;

    /// Register a message type name string and get a local ID for it.
    ///
    /// If the string is already registered, the returned ID will be the previously-assigned one.
    fn register_type<T>(&self, name: T) -> Result<LocalId<MessageTypeId>>
    where
        T: Into<MessageTypeName> + Clone,
    {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        let name: MessageTypeName = name.into();
        match dispatcher.register_type(name.clone())? {
            RegisterMapping::Found(id) => Ok(id),
            RegisterMapping::NewMapping(id) => {
                eprintln!("New mapping (coming from our side): {:?} -> {:?}", name, id);
                let mut endpoints = self.connection_core().endpoints.lock()?;
                for ep in endpoints.iter_mut().flatten() {
                    ep.new_local_id(name.clone(), id)?;
                }
                Ok(id)
            }
        }
    }

    /// Register a sender name string and get a local ID for it.
    ///
    /// If the string is already registered, the returned ID will be the previously-assigned one.
    fn register_sender<T>(&self, name: T) -> Result<LocalId<SenderId>>
    where
        T: Into<SenderName> + Clone,
    {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        match dispatcher.register_sender(name.clone())? {
            RegisterMapping::Found(id) => Ok(id),
            RegisterMapping::NewMapping(id) => {
                let mut endpoints = self.connection_core().endpoints.lock()?;
                for ep in endpoints.iter_mut().flatten() {
                    ep.new_local_id(name.clone(), id)?;
                }
                Ok(id)
            }
        }
    }

    /// Add a generic handler, with optional filters on message type and sender.
    ///
    /// Returns a struct usable to remove the handler later.
    fn add_handler(
        &self,
        handler: Box<dyn Handler + Send>,
        message_type_filter: Option<LocalId<MessageTypeId>>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandle> {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        dispatcher.add_handler(handler, message_type_filter, sender_filter)
    }

    /// Add a "typed" handler, with optional filters on sender.
    ///
    /// The message type filter is automatically populated based on the TypedHandler trait.
    ///
    /// Returns a struct usable to remove the handler later.
    fn add_typed_handler<T: 'static>(
        &self,
        handler: Box<T>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandle>
    where
        T: TypedHandler + Handler + Sized,
    {
        let message_type_filter = match T::Item::MESSAGE_IDENTIFIER {
            MessageTypeIdentifier::UserMessageName(name) => Some(self.register_type(name)?),
            MessageTypeIdentifier::SystemMessageId(id) => Some(LocalId(id)),
        };
        self.add_handler(handler, message_type_filter, sender_filter)
    }

    /// Remove a handler previously added with add_handler() or add_typed_handler()
    fn remove_handler(&self, handler_handle: HandlerHandle) -> Result<()> {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        dispatcher.remove_handler(handler_handle)
    }

    /// Pack a message to send to all connected endpoints.
    ///
    /// May not actually send immediately, might need to poll the connection somehow.
    fn pack_message<T>(&self, msg: TypedMessage<T>, class: ClassOfService) -> Result<()>
    where
        T: TypedMessageBody + BufferTo,
    {
        let generic_msg = GenericMessage::try_from(msg)?;

        let mut endpoints = self.connection_core().endpoints.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.buffer_generic_message(generic_msg.clone(), class)?;
        }
        Ok(())
    }

    /// Pack a message body to send to all connected endpoints.
    ///
    /// Generates the header automatically from the supplied parameters as well as
    /// the MESSAGE_IDENTIFIER constant in the TypedMessageBody implementation.
    ///
    /// May not actually send immediately, might need to poll the connection somehow.
    fn pack_message_body<T: TypedMessageBody>(
        &self,
        timeval: Option<TimeVal>,
        sender: LocalId<SenderId>,
        body: T,
        class: ClassOfService,
    ) -> Result<()>
    where
        T: TypedMessageBody + BufferTo,
    {
        let message_type = match T::MESSAGE_IDENTIFIER {
            MessageTypeIdentifier::UserMessageName(name) => self.register_type(name)?,
            MessageTypeIdentifier::SystemMessageId(id) => LocalId(id),
        };
        let message: TypedMessage<T> = TypedMessage::new(timeval, message_type, sender, body);
        self.pack_message(message, class)
    }

    /// Pack an ID description (either message type or sender) on all endpoints.
    ///
    /// May not actually send immediately, might need to poll the connection somehow.
    fn pack_description<T>(&self, id: LocalId<T>) -> Result<()>
    where
        T: UnwrappedId + IdWithDescription,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let mut endpoints = self.connection_core().endpoints.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.pack_description(id)?;
        }
        Ok(())
    }

    /// Pack all message type and sender descriptions on all endpoints.
    ///
    /// May not actually send immediately, might need to poll the connection somehow.
    fn pack_all_descriptions(&self) -> Result<()> {
        let mut endpoints = self.connection_core().endpoints.lock()?;
        let dispatcher = self.connection_core().type_dispatcher.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.pack_all_descriptions(&dispatcher)?;
        }
        Ok(())
    }

    /// Gets a reference-counted handle to the mutex-protected endpoint vector.
    fn endpoints(&self) -> SharedEndpointVec<Self::SpecificEndpoint> {
        Arc::clone(&self.connection_core().endpoints)
    }

    /// Gets a reference-counted handle to the mutex-protected type dispatcher.
    fn dispatcher(&self) -> Arc<Mutex<TypeDispatcher>> {
        Arc::clone(&self.connection_core().type_dispatcher)
    }
}

#[derive(Debug)]
pub struct ConnectionCore<EP>
where
    EP: Endpoint + EndpointGeneric,
{
    pub(crate) endpoints: SharedEndpointVec<EP>,
    pub(crate) type_dispatcher: Arc<Mutex<TypeDispatcher>>,
    remote_log_names: LogFileNames,
    local_log_names: LogFileNames,
}
impl<EP> ConnectionCore<EP>
where
    EP: Endpoint + EndpointGeneric,
{
    pub fn new(
        endpoints: Vec<Option<EP>>,
        local_log_names: Option<LogFileNames>,
        remote_log_names: Option<LogFileNames>,
    ) -> ConnectionCore<EP> {
        ConnectionCore {
            endpoints: Arc::new(Mutex::new(endpoints)),
            type_dispatcher: Arc::new(Mutex::new(TypeDispatcher::new())),
            remote_log_names: LogFileNames::from(remote_log_names),
            local_log_names: LogFileNames::from(local_log_names),
        }
    }
}
