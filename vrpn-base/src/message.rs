// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::{
    time::TimeVal,
    types::{BaseTypeSafeId, SenderId, SequenceNumber, TypeId},
};

/// Header information for a message.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageHeader {
    pub time: TimeVal,
    pub message_type: TypeId,
    pub sender: SenderId,
}

impl MessageHeader {
    pub fn new(time: Option<TimeVal>, message_type: TypeId, sender: SenderId) -> MessageHeader {
        MessageHeader {
            time: time.unwrap_or_else(|| TimeVal::get_time_of_day()),
            message_type,
            sender,
        }
    }
}

/// A message with header information, almost ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Message<T> {
    pub header: MessageHeader,
    pub body: T,
}

pub type GenericMessage = Message<GenericBody>;

impl<T> Message<T> {
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

    pub fn from_header_and_body(header: MessageHeader, body: T) -> Message<T> {
        Message { header, body }
    }

    pub fn add_sequence_number(self, sequence_number: SequenceNumber) -> SequencedMessage<T> {
        SequencedMessage {
            message: self,
            sequence_number,
        }
    }
}

/// A message with header information and sequence number, ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequencedMessage<T> {
    pub message: Message<T>,
    pub sequence_number: SequenceNumber,
}

pub type SequencedGenericMessage = SequencedMessage<GenericBody>;

impl<T> SequencedMessage<T> {
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

impl<T> From<SequencedMessage<T>> for Message<T> {
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

/// Body struct for use in Message<T> for sender/type descriptions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InnerDescription {
    pub name: Bytes,
}

impl InnerDescription {
    pub fn new(name: Bytes) -> InnerDescription {
        InnerDescription { name }
    }
}

impl Message<InnerDescription> {
    pub fn into_typed_description<T: BaseTypeSafeId>(self) -> Description<T> {
        use super::types::TypeSafeId;
        Description::new(T::new(self.header.sender.get()), self.body.name)
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

impl<T: BaseTypeSafeId> From<Description<T>> for Message<InnerDescription> {
    fn from(v: Description<T>) -> Message<InnerDescription> {
        Message::new(
            None,
            T::description_type(),
            SenderId(v.which.get()),
            InnerDescription { name: v.name },
        )
    }
}
