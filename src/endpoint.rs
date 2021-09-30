// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::convert::{TryFrom, TryInto};

use crate::{
    buffer_unbuffer::BufferTo,
    data_types::{
        constants,
        descriptions::{InnerDescription, UdpInnerDescription},
        id_types::*,
        message::Message,
        name_types::NameIntoBytes,
        ClassOfService, Description, GenericMessage, IdWithNameAndDescription, LogFileNames,
        MessageHeader, MessageTypeId, MessageTypeName, SenderName, TypedMessage, TypedMessageBody,
        UdpDescription,
    },
    translation_table,
    type_dispatcher::IntoDescriptionMessage,
    MatchingTable, Result, TranslationTables, TypeDispatcher, VrpnError,
};

/// These are all "system commands".
/// They are converted from system messages by Endpoint::handle_message_as_system
/// (and thus Endpoint::passthrough_nonsystem_message).
///
/// The commands enumerated that aren't Extended are handled by handle_system_command.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SystemCommand {
    SenderDescription(Description<SenderId>),
    TypeDescription(Description<MessageTypeId>),
    Extended(ExtendedSystemCommand),
}

/// These are the system commands not handled by Endpoint::handle_system_command
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ExtendedSystemCommand {
    UdpDescription(UdpDescription),
    LogDescription(LogFileNames),
    DisconnectMessage,
}

/// Parse a "system" message (for which message_type.is_system_message() returns true).
///
/// Call from within your dispatch function once you've recognized that a message is a system message.
pub fn parse_system_message(msg: GenericMessage) -> Result<SystemCommand> {
    if !msg.is_system_message() {
        return Err(VrpnError::NotSystemMessage);
    }
    Ok(match msg.header.message_type {
        constants::TYPE_DESCRIPTION => {
            let msg: TypedMessage<InnerDescription<MessageTypeId>> = TypedMessage::try_from(&msg)?;
            SystemCommand::TypeDescription(msg.into())
        }
        constants::SENDER_DESCRIPTION => {
            let msg: TypedMessage<InnerDescription<SenderId>> = TypedMessage::try_from(&msg)?;
            SystemCommand::SenderDescription(msg.into())
        }
        constants::UDP_DESCRIPTION => {
            let msg: TypedMessage<UdpInnerDescription> = TypedMessage::try_from(&msg)?;
            SystemCommand::Extended(ExtendedSystemCommand::UdpDescription(msg.into()))
        }
        constants::LOG_DESCRIPTION => {
            let msg: TypedMessage<LogFileNames> = TypedMessage::try_from(&msg)?;
            SystemCommand::Extended(ExtendedSystemCommand::LogDescription(msg.body))
        }
        constants::DISCONNECT_MESSAGE => {
            SystemCommand::Extended(ExtendedSystemCommand::DisconnectMessage)
        }
        _ => {
            return Err(VrpnError::UnrecognizedSystemMessage(
                msg.header.message_type.get(),
            ));
        }
    })
}

/// Apply the changes from a system command to your TypeDispatcher and TranslationTables.
///
/// Passes through any extended commands.
pub fn handle_system_command(
    dispatcher: &mut TypeDispatcher,
    translation_tables: &mut TranslationTables,
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
            let _ =
                translation_tables.add_remote_entry(desc.name, RemoteId(desc.which), local_id)?;
            Ok(None)
        }
        SystemCommand::TypeDescription(desc) => {
            let local_id = dispatcher
                .register_type(MessageTypeName(desc.name.clone()))?
                .get();
            eprintln!(
                "Registering type {:?}: local {:?} = remote {:?}",
                desc.name, local_id, desc.which
            );
            let _ =
                translation_tables.add_remote_entry(desc.name, RemoteId(desc.which), local_id)?;
            Ok(None)
        }
        SystemCommand::Extended(cmd) => Ok(Some(cmd)),
    }
}

/// An endpoint for communication.
///
/// An endpoint must own:
/// - a set of `TranslationTables`
pub trait Endpoint {
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

    /// Pack all descriptions from the dispatcher and send them.
    fn send_all_descriptions(&mut self, dispatcher: &TypeDispatcher) -> Result<()> {
        for msg in dispatcher.pack_all_descriptions()? {
            self.buffer_generic_message(msg, ClassOfService::RELIABLE)?;
        }
        Ok(())
    }

    // fn clear_other_senders_and_types(&mut self) {
    //     self.translation_tables_mut().clear();
    // }
}

/// Endpoint-related methods that must be separate from the main Endpoint trait,
/// because they are generic/have type parameters. (or depend on those methods)
pub trait EndpointGeneric: Endpoint {
    fn buffer_message<T>(&mut self, msg: TypedMessage<T>, class: ClassOfService) -> Result<()>
    where
        T: BufferTo + TypedMessageBody;

    fn new_local_id<T, V>(&mut self, name: V, local_id: LocalId<T>) -> Result<()>
    where
        T: IdWithNameAndDescription,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
        V: NameIntoBytes;

    fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: UnwrappedId,
        TranslationTables: MatchingTable<T>;

    fn map_remote_message_to_local(&self, msg: GenericMessage) -> Result<GenericMessage>;
}

pub trait PackDescription {
    fn pack_description<T>(&self, local_id: LocalId<T>) -> Result<GenericMessage>
    where
        T: IdWithNameAndDescription,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: translation_table::MatchingTable<T>;
}

impl PackDescription for TranslationTables {
    fn pack_description<T>(&self, local_id: LocalId<T>) -> Result<GenericMessage>
    where
        T: IdWithNameAndDescription,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: translation_table::MatchingTable<T>,
    {
        self.find_by_local_id(local_id)
            .ok_or_else(|| VrpnError::InvalidId(local_id.get()))?
            .clone()
            .try_into()
    }
}

impl<U> EndpointGeneric for U
where
    U: Endpoint,
{
    fn buffer_message<T>(&mut self, msg: TypedMessage<T>, class: ClassOfService) -> Result<()>
    where
        T: BufferTo + TypedMessageBody,
    {
        let generic_msg = GenericMessage::try_from(msg)?;
        self.buffer_generic_message(generic_msg, class)
    }

    fn new_local_id<T, V>(&mut self, name: V, local_id: LocalId<T>) -> Result<()>
    where
        T: IdWithNameAndDescription,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
        V: NameIntoBytes,
    {
        let name = name.into_bytes();
        self.translation_tables_mut()
            .add_local_id(name.clone(), local_id);

        self.buffer_generic_message(
            local_id.into_description_message(name)?,
            ClassOfService::RELIABLE,
        )
    }

    /// Convert remote sender/type ID to local sender/type ID
    fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: UnwrappedId,
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
            let LocalId(new_type) = self.map_to_local_id(remote_type).ok_or_else(|| {
                VrpnError::OtherMessage("Could not map sender to local".to_string())
            })?;
            let remote_sender = RemoteId(msg.header.sender);
            let LocalId(new_sender) = self.map_to_local_id(remote_sender).ok_or_else(|| {
                VrpnError::OtherMessage("Could not map type to local".to_string())
            })?;

            // eprintln!("user message: {:?}", msg.header);
            let msg = GenericMessage::from_header_and_body(
                MessageHeader::new(Some(msg.header.time), new_type, new_sender),
                msg.body,
            );
            Ok(msg)
        }
    }
}
