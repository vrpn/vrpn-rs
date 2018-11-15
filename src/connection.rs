// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    descriptions::InnerDescription, type_dispatcher::HandlerHandle, BaseTypeSafeId, Buffer,
    ClassOfService, Endpoint, EndpointGeneric, Handler, LocalId, LogFileNames, MatchingTable,
    Message, MessageTypeIdentifier, RegisterMapping, Result, SenderId, SenderName,
    TranslationTables, TypeDispatcher, TypeId, TypeName, TypedHandler, TypedMessageBody,
};
use std::sync::{Arc, Mutex};

pub type EndpointVec<EP> = Vec<Option<EP>>;
pub type SharedEndpointVec<EP> = Arc<Mutex<EndpointVec<EP>>>;

pub trait Connection {
    type SpecificEndpoint: Endpoint + EndpointGeneric;

    /// Access the ConnectionCore nested struct.
    ///
    /// This is the main required method for this trait.
    fn connection_core(&self) -> &ConnectionCore<Self::SpecificEndpoint>;

    fn register_type<T>(&self, name: T) -> Result<LocalId<TypeId>>
    where
        T: Into<TypeName> + Clone,
    {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        let name: TypeName = name.into();
        match dispatcher.register_type(name.clone())? {
            RegisterMapping::Found(id) => {
                eprintln!("Type already defined as {:?} -> {:?}", name.clone(), id);
                Ok(id)
            }
            RegisterMapping::NewMapping(id) => {
                eprintln!("New mapping: {:?} -> {:?}", name.clone(), id);
                let mut endpoints = self.connection_core().endpoints.lock()?;
                for ep in endpoints.iter_mut().flatten() {
                    ep.new_local_id(name.clone(), id)?;
                }
                Ok(id)
            }
        }
    }

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

    fn add_handler(
        &self,
        handler: Box<dyn Handler>,
        message_type_filter: Option<LocalId<TypeId>>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandle> {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        dispatcher.add_handler(handler, message_type_filter, sender_filter)
    }

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

    fn remove_handler(&self, handler_handle: HandlerHandle) -> Result<()> {
        let mut dispatcher = self.connection_core().type_dispatcher.lock()?;
        dispatcher.remove_handler(handler_handle)
    }

    fn pack_message<T>(&self, msg: Message<T>, class: ClassOfService) -> Result<()>
    where
        T: TypedMessageBody + Buffer,
    {
        let generic_msg = msg.try_into_generic()?;

        let mut endpoints = self.connection_core().endpoints.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.buffer_generic_message(generic_msg.clone(), class)?;
        }
        Ok(())
    }

    fn pack_description<T>(&self, id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let mut endpoints = self.connection_core().endpoints.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.pack_description(id)?;
        }
        Ok(())
    }

    fn pack_all_descriptions(&self) -> Result<()> {
        let mut endpoints = self.connection_core().endpoints.lock()?;
        let dispatcher = self.connection_core().type_dispatcher.lock()?;
        for ep in endpoints.iter_mut().flatten() {
            ep.pack_all_descriptions(&dispatcher)?;
        }
        Ok(())
    }

    fn endpoints(&self) -> SharedEndpointVec<Self::SpecificEndpoint> {
        Arc::clone(&self.connection_core().endpoints)
    }

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
