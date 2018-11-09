// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::{
    constants, BaseTypeSafeId, IdType, RemoteId, SenderId, SequenceNumber, StaticTypeName, TimeVal,
    TypeId, TypeSafeId,
};
use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
};

/// Empty trait used to indicate types that can be placed in a message body.
pub trait MessageBody {}

/// The identification used for a typed message body type.
pub enum MessageTypeIdentifier {
    /// User message types are identified by a string which is dynamically associated
    /// with an ID on each side.
    UserMessageName(StaticTypeName),

    /// System message types are identified by a constant, negative message type ID.
    ///
    /// TODO: find a way to assert/enforce that this is negative - maybe a SystemTypeId type?
    SystemMessageId(TypeId),
}

/// Trait for typed message bodies.
///
pub trait TypedMessageBody {
    /// The name string (for user messages) or type ID (for system messages) used to identify this message type.
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier;
}

impl<T> MessageBody for T where T: TypedMessageBody {}

/// Header information for a message.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageHeader {
    time: TimeVal,
    message_type: TypeId,
    sender: SenderId,
}

impl MessageHeader {
    pub fn new(time: Option<TimeVal>, message_type: TypeId, sender: SenderId) -> MessageHeader {
        MessageHeader {
            time: time.unwrap_or_else(|| TimeVal::get_time_of_day()),
            message_type,
            sender,
        }
    }
    pub fn time(&self) -> &TimeVal {
        &self.time
    }
    pub fn message_type(&self) -> TypeId {
        self.message_type
    }
    pub fn sender(&self) -> SenderId {
        self.sender
    }
}

/// A message with header information, almost ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Message<T: MessageBody> {
    pub header: MessageHeader,
    pub body: T,
}

pub type GenericMessage = Message<GenericBody>;

impl<T: MessageBody> Message<T> {
    pub fn new(
        time: Option<TimeVal>,
        message_type: TypeId,
        sender: SenderId,
        body: T,
    ) -> Message<T> {
        Message {
            header: MessageHeader::new(time, message_type, sender),
            body,
        }
    }

    /// Create a message by combining a header and a body.
    pub fn from_header_and_body(header: MessageHeader, body: T) -> Message<T> {
        Message { header, body }
    }

    /// Consumes this message and returns a new SequencedMessage, which the supplied sequence number has been added to.
    pub fn into_sequenced_message(self, sequence_number: SequenceNumber) -> SequencedMessage<T> {
        SequencedMessage {
            message: self,
            sequence_number,
        }
    }

    /// true if the message type indicates that it is a "system" message (type ID < 0)
    pub fn is_system_message(&self) -> bool {
        self.header.message_type.is_system_message()
    }
}

/// A message with header information and sequence number, ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequencedMessage<T: MessageBody> {
    pub message: Message<T>,
    pub sequence_number: SequenceNumber,
}

pub type SequencedGenericMessage = SequencedMessage<GenericBody>;

impl<T: MessageBody> SequencedMessage<T> {
    pub fn new(
        time: Option<TimeVal>,
        message_type: TypeId,
        sender: SenderId,
        body: T,
        sequence_number: SequenceNumber,
    ) -> SequencedMessage<T> {
        SequencedMessage {
            message: Message::new(time, message_type, sender, body),
            sequence_number,
        }
    }
}

impl<T: MessageBody> From<SequencedMessage<T>> for Message<T> {
    fn from(v: SequencedMessage<T>) -> Message<T> {
        v.message
    }
}

/// Generic body struct used in unbuffering process, before dispatch on type to fully decode.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GenericBody {
    pub inner: Bytes,
}

impl GenericBody {
    pub fn new(inner: Bytes) -> GenericBody {
        GenericBody { inner }
    }
}

impl MessageBody for GenericBody {}

/// Body struct for use in Message<T> for sender/type descriptions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InnerDescription<T: BaseTypeSafeId> {
    name: Bytes,
    phantom: PhantomData<T>,
}

impl<T: BaseTypeSafeId> InnerDescription<T> {
    pub fn new(name: Bytes) -> InnerDescription<T> {
        InnerDescription {
            name,
            phantom: PhantomData,
        }
    }

    pub fn name(&self) -> &Bytes {
        &self.name
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
        T::new(self.header.sender.get())
    }
}

impl<T> From<Message<InnerDescription<T>>> for Description<T>
where
    T: BaseTypeSafeId,
    InnerDescription<T>: TypedMessageBody,
{
    fn from(v: Message<InnerDescription<T>>) -> Description<T> {
        Description::new(v.which(), v.body.name)
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
        self.header.sender.get() as u16
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
