// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    time::TimeVal,
    types::{BaseTypeSafeId, SenderId, SequenceNumber, TypeId},
};
use bytes::Bytes;

/// A message with header information, ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Message<T> {
    pub time: TimeVal,
    pub message_type: TypeId,
    pub sender: SenderId,
    pub data: T,
    pub sequence_number: Option<SequenceNumber>,
}

pub type GenericMessage = Message<GenericBody>;

impl<T> Message<T> {
    pub fn new(
        time: Option<TimeVal>,
        message_type: TypeId,
        sender: SenderId,
        data: T,
        sequence_number: Option<SequenceNumber>,
    ) -> Message<T> {
        Message {
            time: time.unwrap_or_else(|| TimeVal::get_time_of_day()),
            message_type,
            sender,
            data,
            sequence_number,
        }
    }

    pub fn from_parsed_generic<U>(generic: Message<GenericBody>, data: U) -> Message<U> {
        Message {
            time: generic.time,
            message_type: generic.message_type,
            sender: generic.sender,
            data,
            sequence_number: generic.sequence_number,
        }
    }
}

/// Generic body struct used in unbuffering process, before dispatch on type to fully decode.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GenericBody {
    pub body_bytes: Bytes,
}

impl GenericBody {
    pub fn new(body_bytes: Bytes) -> GenericBody {
        GenericBody { body_bytes }
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
            None,
        )
    }
}
