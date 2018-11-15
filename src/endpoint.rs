// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::{
    constants,
    descriptions::{InnerDescription, UdpDescription, UdpInnerDescription},
    BaseTypeSafeId, BaseTypeSafeIdName, Buffer, ClassOfService, Description, Error, GenericMessage,
    IntoId, LocalId, LogFileNames, MatchingTable, Message, RemoteId, Result, SenderId,
    ServiceFlags, TranslationTables, TypeDispatcher, TypeId, TypeSafeId, TypedMessageBody,
};
use downcast_rs::Downcast;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SystemMessage {
    SenderDescription(Description<SenderId>),
    UdpDescription(UdpDescription),
    TypeDescription(Description<TypeId>),
    LogDescription(LogFileNames),
    DisconnectMessage,
}

pub trait Endpoint: Downcast {
    /// Access the translation tables.
    fn translation_tables(&self) -> &TranslationTables;
    /// Access the translation tables mutably.
    fn translation_tables_mut(&mut self) -> &mut TranslationTables;

    /// Send a system change message.
    ///
    /// Implementation should use interior mutability.
    fn send_system_change(&self, message: SystemMessage) -> Result<()>;

    /// Queue up a generic message for sending.
    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()>;

    /// Handle a "system" message (for which message_type.is_system_message() returns true).
    ///
    /// Call from within your dispatch function once you've recognized that a message is a system message.
    fn handle_system_message(&self, msg: GenericMessage) -> Result<()> {
        if !msg.is_system_message() {
            Err(Error::NotSystemMessage)?;
        }
        match msg.header.message_type {
            constants::TYPE_DESCRIPTION => {
                let msg: Message<InnerDescription<TypeId>> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemMessage::TypeDescription(msg.into()))?;
            }
            constants::SENDER_DESCRIPTION => {
                let msg: Message<InnerDescription<SenderId>> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemMessage::SenderDescription(msg.into()))?;
            }
            constants::UDP_DESCRIPTION => {
                let msg: Message<UdpInnerDescription> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemMessage::UdpDescription(msg.into()))?;
            }
            constants::LOG_DESCRIPTION => {
                let msg: Message<LogFileNames> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemMessage::LogDescription(msg.body))?;
            }
            constants::DISCONNECT_MESSAGE => {
                self.send_system_change(SystemMessage::DisconnectMessage)?;
            }
            _ => {
                Err(Error::UnrecognizedSystemMessage(
                    msg.header.message_type.get(),
                ))?;
            }
        }
        Ok(())
    }

    fn pack_all_descriptions(&mut self, dispatcher: &TypeDispatcher) -> Result<()> {
        let mut messages = Vec::new();
        for (id, name) in dispatcher.senders_iter() {
            let desc_msg = Message::from(Description::new(id.into_id(), name.0.clone()));
            messages.push(desc_msg.try_into_generic()?);
        }
        for (id, name) in dispatcher.types_iter() {
            let desc_msg = Message::from(Description::new(id.into_id(), name.0.clone()));
            messages.push(desc_msg.try_into_generic()?);
        }
        for msg in messages.into_iter() {
            self.buffer_generic_message(msg, ClassOfService::from(ServiceFlags::RELIABLE))?;
        }
        Ok(())
    }
    fn clear_other_senders_and_types(&mut self) {
        self.translation_tables_mut().clear();
    }
}

impl_downcast!(Endpoint);

/// Endpoint-related methods that must be separate from the main Endpoint trait,
/// because they are generic/have type parameters.
pub trait EndpointGeneric: Endpoint {
    fn buffer_message<T>(&mut self, msg: Message<T>, class: ClassOfService) -> Result<()>
    where
        T: Buffer + TypedMessageBody;

    fn pack_description_impl<T>(&mut self, name: Bytes, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>;

    fn pack_description<T>(&mut self, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>;

    fn new_local_id<T, V>(&mut self, name: V, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeIdName + BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
        V: Into<<T as BaseTypeSafeIdName>::Name>;
    fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: BaseTypeSafeId,
        TranslationTables: MatchingTable<T>;
}

impl<U> EndpointGeneric for U
where
    U: Endpoint,
{
    fn buffer_message<T>(&mut self, msg: Message<T>, class: ClassOfService) -> Result<()>
    where
        T: Buffer + TypedMessageBody,
    {
        let generic_msg = msg.try_into_generic()?;
        self.buffer_generic_message(generic_msg, class)
    }
    fn pack_description_impl<T>(&mut self, name: Bytes, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let desc_msg = Message::from(Description::new(local_id.0, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map(|_| ())
    }

    fn pack_description<T>(&mut self, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let name = self
            .translation_tables()
            .find_by_local_id(local_id)
            .ok_or_else(|| Error::InvalidId(local_id.get()))
            .and_then(|entry| Ok(entry.name().clone()))?;

        self.pack_description_impl(name, local_id)
    }

    fn new_local_id<T, V>(&mut self, name: V, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeIdName + BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
        V: Into<<T as BaseTypeSafeIdName>::Name>,
    {
        let name: <T as BaseTypeSafeIdName>::Name = name.into();
        let name: Bytes = name.into();
        self.translation_tables_mut()
            .add_local_id(name.clone(), local_id);
        self.pack_description_impl(name, local_id)
    }

    /// Convert remote sender/type ID to local sender/type ID
    fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: BaseTypeSafeId,
        TranslationTables: MatchingTable<T>,
    {
        self.translation_tables()
            .map_to_local_id(remote_id)
            .ok()
            .unwrap_or_default()
    }
}
