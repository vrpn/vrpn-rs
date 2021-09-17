// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::prelude::*;
use crate::{
    constants, length_prefixed, BaseTypeSafeId, Buffer, BufferSize, EmptyResult, Error, IdType,
    Message, MessageTypeIdentifier, Result, SenderId, TypeId, TypedMessageBody, Unbuffer,
};
use bytes::{buf::IntoIter, Buf, BufMut, Bytes};
use std::io::BufRead;
use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
};

/// Body struct for use in Message<T> for sender/type descriptions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InnerDescription<T: BaseTypeSafeId> {
    pub(crate) name: Bytes,
    phantom: PhantomData<T>,
}

impl<T: BaseTypeSafeId> InnerDescription<T> {
    pub fn new(name: Bytes) -> InnerDescription<T> {
        InnerDescription {
            name,
            phantom: PhantomData,
        }
    }
}

impl TypedMessageBody for InnerDescription<SenderId> {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::SystemMessageId(constants::SENDER_DESCRIPTION);
}
impl TypedMessageBody for InnerDescription<TypeId> {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::SystemMessageId(constants::TYPE_DESCRIPTION);
}

impl<T> Message<InnerDescription<T>>
where
    T: BaseTypeSafeId,
    InnerDescription<T>: TypedMessageBody,
{
    fn which(&self) -> T {
        T::new(self.header.sender.0)
    }
}

impl<T> From<Message<InnerDescription<T>>> for Description<T>
where
    T: BaseTypeSafeId,
    InnerDescription<T>: TypedMessageBody,
{
    fn from(v: Message<InnerDescription<T>>) -> Description<T> {
        let id: T = v.which();
        Description::new(id, v.body.name)
    }
}

/// Typed description of a sender or type.
///
/// Converted to a Message<InnerDescription> before being sent.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Description<T: BaseTypeSafeId> {
    /// The ID
    pub which: T,
    /// The name associated with the ID (no null termination in this string)
    pub name: Bytes,
}

impl<T: BaseTypeSafeId> Description<T> {
    pub fn new(which: T, name: Bytes) -> Description<T> {
        Description { which, name }
    }
}

impl<T> From<Description<T>> for Message<InnerDescription<T>>
where
    T: BaseTypeSafeId,
    InnerDescription<T>: TypedMessageBody,
{
    fn from(v: Description<T>) -> Message<InnerDescription<T>> {
        Message::new(
            None,
            T::description_type(),
            SenderId(v.which.get()),
            InnerDescription::new(v.name),
        )
    }
}

/// A more usable description of the UDP_DESCRIPTION system message,
/// with the address parsed and the port loaded as well.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UdpDescription {
    pub socket_address: SocketAddr,
}

impl UdpDescription {
    pub fn new(socket_address: SocketAddr) -> UdpDescription {
        UdpDescription { socket_address }
    }
}

/// MessageBody-implementing structure for UDP_DESCRIPTION system messages.
///
/// The port is carried in the "sender" field.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UdpInnerDescription {
    pub address: IpAddr,
}
impl UdpInnerDescription {
    pub fn new(address: IpAddr) -> UdpInnerDescription {
        UdpInnerDescription { address }
    }
}

impl TypedMessageBody for UdpInnerDescription {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::SystemMessageId(constants::UDP_DESCRIPTION);
}

impl Message<UdpInnerDescription> {
    fn port(&self) -> u16 {
        self.header.sender.0 as u16
    }
}

impl From<Message<UdpInnerDescription>> for UdpDescription {
    fn from(v: Message<UdpInnerDescription>) -> UdpDescription {
        UdpDescription {
            socket_address: SocketAddr::new(v.body.address, v.port()),
        }
    }
}

impl From<UdpDescription> for Message<UdpInnerDescription> {
    fn from(v: UdpDescription) -> Message<UdpInnerDescription> {
        Message::new(
            None,
            constants::UDP_DESCRIPTION,
            SenderId(v.socket_address.port() as IdType),
            UdpInnerDescription::new(v.socket_address.ip()),
        )
    }
}

impl<T: BaseTypeSafeId> BufferSize for InnerDescription<T> {
    fn buffer_size(&self) -> usize {
        length_prefixed::buffer_size(
            self.name.as_ref(),
            length_prefixed::NullTermination::AddTrailingNull,
        )
    }
}

impl<U: BaseTypeSafeId> Buffer for InnerDescription<U> {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        length_prefixed::buffer_string(
            self.name.as_ref(),
            buf,
            length_prefixed::NullTermination::AddTrailingNull,
            length_prefixed::LengthBehavior::IncludeNull,
        )
    }
}

impl<T: BaseTypeSafeId> Unbuffer for InnerDescription<T> {
    fn unbuffer_ref<U: Buf>(buf: &mut U) -> Result<Self> {
        length_prefixed::unbuffer_string(buf).map(InnerDescription::new)
    }
}

impl Unbuffer for UdpInnerDescription {
    fn unbuffer_ref<T: Buf>(buf: &mut T) -> Result<Self> {
        let mut ip_buf: Vec<u8> = Vec::default();
        let length = buf.reader().read_until(0, &mut ip_buf)?;
        // let ip_buf: Vec<u8> = buf.into_iter().take_while(|b| **b != 0).cloned().collect();
        let ip_str = String::from_utf8_lossy(&ip_buf);
        let addr: IpAddr = ip_str
            .parse()
            .map_err(|e| Error::OtherMessage(format!("ip address parse error: {}", e)))?;
        buf.advance(ip_buf.len());

        Ok(UdpInnerDescription::new(addr))
    }
}

impl BufferSize for UdpInnerDescription {
    fn buffer_size(&self) -> usize {
        self.address.to_string().len() + 1
    }
}
impl Buffer for UdpInnerDescription {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        let addr_str = self.address.to_string();
        if buf.remaining_mut() < (addr_str.len() + 1) {
            return Err(Error::OutOfBuffer);
        }
        buf.put(addr_str.as_bytes());
        buf.put_u8(0);
        Ok(())
    }
}
