// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants,
    descriptions::{InnerDescription, UdpDescription, UdpInnerDescription},
    BaseTypeSafeId, BaseTypeSafeIdName, Buffer, ClassOfService, Description, Error, GenericMessage,
    IntoId, LocalId, LogFileNames, MatchingTable, Message, MessageHeader, RemoteId, Result,
    SenderId, SenderName, ServiceFlags, TranslationTables, TypeDispatcher, TypeId, TypeName,
    TypeSafeId, TypedMessageBody,
};
use bytes::Bytes;
use downcast_rs::Downcast;

/// These are all "system commands".
/// They are converted from system messages by Endpoint::handle_message_as_system
/// (and thus Endpoint::passthrough_nonsystem_message).
///
/// The commands enumerated that aren't Extended are handled by the default implementation of Endpoint::handle_system_command.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SystemCommand {
    SenderDescription(Description<SenderId>),
    TypeDescription(Description<TypeId>),
    Extended(ExtendedSystemCommand),
}

/// These are the system commands not handled by Endpoint::handle_system_command
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ExtendedSystemCommand {
    UdpDescription(UdpDescription),
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
    fn send_system_change(&self, message: SystemCommand) -> Result<()>;

    /// Queue up a generic message for sending.
    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()>;

    /// Handle a "system" message (for which message_type.is_system_message() returns true).
    ///
    /// Call from within your dispatch function once you've recognized that a message is a system message.
    fn handle_message_as_system(&self, msg: GenericMessage) -> Result<()> {
        if !msg.is_system_message() {
            Err(Error::NotSystemMessage)?;
        }
        match msg.header.message_type {
            constants::TYPE_DESCRIPTION => {
                let msg: Message<InnerDescription<TypeId>> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemCommand::TypeDescription(msg.into()))?;
            }
            constants::SENDER_DESCRIPTION => {
                let msg: Message<InnerDescription<SenderId>> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemCommand::SenderDescription(msg.into()))?;
            }
            constants::UDP_DESCRIPTION => {
                let msg: Message<UdpInnerDescription> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemCommand::Extended(
                    ExtendedSystemCommand::UdpDescription(msg.into()),
                ))?;
            }
            constants::LOG_DESCRIPTION => {
                let msg: Message<LogFileNames> = Message::try_from_generic(&msg)?;
                self.send_system_change(SystemCommand::Extended(
                    ExtendedSystemCommand::LogDescription(msg.body),
                ))?;
            }
            constants::DISCONNECT_MESSAGE => {
                self.send_system_change(SystemCommand::Extended(
                    ExtendedSystemCommand::DisconnectMessage,
                ))?;
            }
            _ => {
                Err(Error::UnrecognizedSystemMessage(
                    msg.header.message_type.get(),
                ))?;
            }
        }
        Ok(())
    }

    /// If a message is a system message, handle it, otherwise pass it through unmodified.
    ///
    /// Call from within your dispatch function, when processing unbuffered messages
    fn passthrough_nonsystem_message(&self, msg: GenericMessage) -> Result<Option<GenericMessage>> {
        if msg.is_system_message() {
            self.handle_message_as_system(msg)?;
            Ok(None)
        } else {
            Ok(Some(msg))
        }
    }

    /// Call from within your dispatch function when you're looping through the contents of your queue of SystemCommand objects.
    ///
    /// Passes through any extended system commands.
    fn handle_system_command(
        &mut self,
        dispatcher: &mut TypeDispatcher,
        system_command: SystemCommand,
    ) -> Result<Option<ExtendedSystemCommand>> {
        match system_command {
            SystemCommand::SenderDescription(desc) => {
                let local_id = dispatcher
                    .register_sender(SenderName(desc.name.clone()))?
                    .get();
                eprintln!(
                    "Registering sender {:?}: local {:?} = remote {:?}",
                    desc.name, local_id, desc.which
                );
                let _ = self.translation_tables_mut().add_remote_entry(
                    desc.name,
                    RemoteId(desc.which),
                    local_id,
                )?;
                Ok(None)
            }
            SystemCommand::TypeDescription(desc) => {
                let local_id = dispatcher.register_type(TypeName(desc.name.clone()))?.get();
                eprintln!(
                    "Registering type {:?}: local {:?} = remote {:?}",
                    desc.name, local_id, desc.which
                );
                let _ = self.translation_tables_mut().add_remote_entry(
                    desc.name,
                    RemoteId(desc.which),
                    local_id,
                )?;
                Ok(None)
            }
            SystemCommand::Extended(cmd) => Ok(Some(cmd)),
        }
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
            self.buffer_generic_message(msg, ClassOfService::from(ServiceFlags::Reliable))?;
        }
        Ok(())
    }

    fn clear_other_senders_and_types(&mut self) {
        self.translation_tables_mut().clear();
    }
}

impl_downcast!(Endpoint);

/// Endpoint-related methods that must be separate from the main Endpoint trait,
/// because they are generic/have type parameters. (or depend on those methods)
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

    fn map_remote_message_to_local(&self, msg: GenericMessage) -> Result<GenericMessage>;
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
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::Reliable))
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

    /// Convert a message with remote sender and type ID to one with local.
    fn map_remote_message_to_local(&self, msg: GenericMessage) -> Result<GenericMessage> {
        if msg.is_system_message() {
            // no mapping applied to system messages
            Ok(msg)
        } else {
            let remote_type = RemoteId(msg.header.message_type);
            let LocalId(new_type) =
                self.map_to_local_id(remote_type)
                    .ok_or(Error::OtherMessage(
                        "Could not map sender to local".to_string(),
                    ))?;
            let remote_sender = RemoteId(msg.header.sender);
            let LocalId(new_sender) =
                self.map_to_local_id(remote_sender)
                    .ok_or(Error::OtherMessage(
                        "Could not map type to local".to_string(),
                    ))?;

            // eprintln!("user message: {:?}", msg.header);
            let msg = Message::from_header_and_body(
                MessageHeader::new(Some(msg.header.time.clone()), new_type, new_sender),
                msg.body,
            );
            Ok(msg)
        }
    }
}
