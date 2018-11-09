// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::TranslationTable;
use downcast_rs::Downcast;
use vrpn_base::{
    constants, ClassOfService, Description, Error, GenericMessage, InnerDescription, LogFileNames,
    Message, Result, SenderId, TypeId, TypeSafeId, TypedMessageBody, UdpDescription,
    UdpInnerDescription,
};
use vrpn_buffer::{make_message_body_generic, unbuffer_typed_message_body, Buffer};

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
        match msg.header.message_type() {
            constants::TYPE_DESCRIPTION => {
                let desc = unbuffer_typed_message_body::<InnerDescription<TypeId>>(msg)?.into();
                self.send_system_change(SystemMessage::TypeDescription(desc))?;
            }
            constants::SENDER_DESCRIPTION => {
                let desc = unbuffer_typed_message_body::<InnerDescription<SenderId>>(msg)?.into();
                self.send_system_change(SystemMessage::SenderDescription(desc))?;
            }
            constants::UDP_DESCRIPTION => {
                let desc = unbuffer_typed_message_body::<UdpInnerDescription>(msg)?.into();
                self.send_system_change(SystemMessage::UdpDescription(desc))?;
            }
            constants::LOG_DESCRIPTION => {
                eprintln!("Handling of LOG_DESCRIPTION not yet implemented");
            }
            constants::DISCONNECT_MESSAGE => {
                eprintln!("Handling of DISCONNECT_MESSAGE not yet implemented");
            }
            _ => {
                Err(Error::UnrecognizedSystemMessage(
                    msg.header.message_type().get(),
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
        let generic_msg = make_message_body_generic(msg)?;
        self.buffer_generic_message(generic_msg, class)
    }
}
