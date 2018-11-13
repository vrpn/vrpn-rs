// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants,
    descriptions::{InnerDescription, UdpDescription, UdpInnerDescription},
    Buffer, ClassOfService, Description, Error, GenericMessage, LogFileNames, Message, Result,
    SenderId, TypeId, TypeSafeId, TypedMessageBody,
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
    /// Send a system change message.
    ///
    /// Implementation should use interior mutability.
    fn send_system_change(&self, message: SystemMessage) -> Result<()>;

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

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()>;
}

impl_downcast!(Endpoint);

pub trait EndpointGeneric {
    fn buffer_message<T>(&mut self, msg: Message<T>, class: ClassOfService) -> Result<()>
    where
        T: Buffer + TypedMessageBody;
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
}
