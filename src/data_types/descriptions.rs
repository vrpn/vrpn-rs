// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes};

use std::{
    io::BufRead,
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
};

use crate::buffer_unbuffer::{
    check_buffer_remaining, BufferResult, BufferSize, BufferTo, UnbufferFrom, UnbufferResult,
};

use super::{
    constants,
    id_types::*,
    length_prefixed,
    name_types::{IdWithNameAndDescription, MessageTypeIdentifier},
    TypedMessage, TypedMessageBody,
};

/// Body struct for use in Message<T> for sender/type descriptions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InnerDescription<I> {
    pub(crate) name: Bytes,
    phantom: PhantomData<I>,
}

impl<I: IdWithNameAndDescription> InnerDescription<I> {
    fn new(name: Bytes) -> InnerDescription<I> {
        InnerDescription {
            name,
            phantom: PhantomData,
        }
    }
}

impl<I: IdWithNameAndDescription> TypedMessage<InnerDescription<I>> {
    fn which(&self) -> I {
        I::new(self.header.sender.0)
    }
}

impl<I: IdWithNameAndDescription> From<TypedMessage<InnerDescription<I>>> for Description<I> {
    fn from(v: TypedMessage<InnerDescription<I>>) -> Description<I> {
        let id: I = v.which();
        Description::from_id_and_name(id, v.body.name)
    }
}

/// Typed description of a sender or type.
///
/// Converted to a Message<InnerDescription> before being sent.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Description<T> {
    /// The ID
    pub which: T,
    /// The name associated with the ID (no null termination in this string)
    pub name: Bytes,
}

impl<I: IdWithNameAndDescription> Description<I> {
    pub fn from_id_and_name(id: I, name: Bytes) -> Description<I> {
        Description {
            which: id.into_id(),
            name,
        }
    }
}

impl<I: IdWithNameAndDescription> From<Description<I>> for TypedMessage<InnerDescription<I>> {
    fn from(v: Description<I>) -> TypedMessage<InnerDescription<I>> {
        TypedMessage::new(
            None,
            I::DESCRIPTION_MESSAGE_TYPE,
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

/// TypedMessageBody-implementing structure for UDP_DESCRIPTION system messages.
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

impl TypedMessage<UdpInnerDescription> {
    fn port(&self) -> u16 {
        self.header.sender.0 as u16
    }
}

impl From<TypedMessage<UdpInnerDescription>> for UdpDescription {
    fn from(v: TypedMessage<UdpInnerDescription>) -> UdpDescription {
        UdpDescription {
            socket_address: SocketAddr::new(v.body.address, v.port()),
        }
    }
}

impl From<UdpDescription> for TypedMessage<UdpInnerDescription> {
    fn from(v: UdpDescription) -> TypedMessage<UdpInnerDescription> {
        TypedMessage::new(
            None,
            constants::UDP_DESCRIPTION,
            SenderId(v.socket_address.port() as IdType),
            UdpInnerDescription::new(v.socket_address.ip()),
        )
    }
}

impl<I> BufferSize for InnerDescription<I>
where
    I: IdWithNameAndDescription,
{
    fn buffer_size(&self) -> usize {
        length_prefixed::buffer_size(
            self.name.as_ref(),
            length_prefixed::NullTermination::AddTrailingNull,
        )
    }
}

impl<I: IdWithNameAndDescription> BufferTo for InnerDescription<I> {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        length_prefixed::buffer_string(
            self.name.as_ref(),
            buf,
            length_prefixed::NullTermination::AddTrailingNull,
            length_prefixed::LengthBehavior::IncludeNull,
        )
    }
}

impl<I: IdWithNameAndDescription> UnbufferFrom for InnerDescription<I> {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        length_prefixed::unbuffer_string(buf).map(InnerDescription::new)
    }
}

impl UnbufferFrom for UdpInnerDescription {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        let mut ip_buf: Vec<u8> = Vec::default();
        // ok to unwrap: a buf reader is infallible. The reader also prevents us from modifying the buffer state.
        let length = buf.reader().read_until(0, &mut ip_buf).unwrap();
        // let ip_buf: Vec<u8> = buf.into_iter().take_while(|b| **b != 0).cloned().collect();
        let ip_str = String::from_utf8_lossy(&ip_buf);
        let addr: IpAddr = ip_str.parse()?;
        buf.advance(length);

        Ok(UdpInnerDescription::new(addr))
    }
}

impl BufferSize for UdpInnerDescription {
    fn buffer_size(&self) -> usize {
        self.address.to_string().len() + 1
    }
}

impl BufferTo for UdpInnerDescription {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        let addr_str = self.address.to_string();
        check_buffer_remaining(buf, addr_str.len() + 1)?;
        buf.put(addr_str.as_bytes());
        buf.put_u8(0);
        Ok(())
    }
}
