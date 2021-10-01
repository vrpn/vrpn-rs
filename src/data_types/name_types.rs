// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Name types used across VRPN

use bytes::Bytes;

use super::{
    constants,
    id_types::{SenderId, UnwrappedId},
    MessageTypeId,
};

/// The identification (name or ID) used for a typed message body type.
#[derive(Debug)]
pub enum MessageTypeIdentifier {
    /// User message types are identified by a string which is dynamically associated
    /// with an ID on each side.
    UserMessageName(StaticMessageTypeName),

    /// System message types are identified by a constant, negative message type ID.
    ///
    // TODO: find a way to assert/enforce that this is negative - maybe a SystemTypeId type?
    SystemMessageId(MessageTypeId),
}

/// A named, unwrapped ID
///
/// Implemented only by MessageTypeId and SenderId
pub trait IdWithNameAndDescription: UnwrappedId {
    type Name: NameIntoBytes;
    const DESCRIPTION_MESSAGE_TYPE: MessageTypeId;
}

pub trait NameIntoBytes {
    fn into_bytes(self) -> Bytes;
}

/// Wrapper for a fixed sender name, as a static byte array.
///
/// Convertible to `SenderName`
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct StaticSenderName(pub &'static [u8]);

impl From<&'static [u8]> for SenderName {
    fn from(val: &'static [u8]) -> SenderName {
        SenderName(Bytes::from_static(val))
    }
}

impl From<&'static [u8]> for StaticSenderName {
    fn from(val: &'static [u8]) -> StaticSenderName {
        StaticSenderName(val)
    }
}

impl std::cmp::PartialEq<SenderName> for StaticSenderName {
    fn eq(&self, other: &SenderName) -> bool {
        Bytes::from_static(self.0) == other.0
    }
}

impl From<StaticSenderName> for Bytes {
    fn from(val: StaticSenderName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl NameIntoBytes for StaticSenderName {
    fn into_bytes(self) -> Bytes {
        SenderName::from(self).into_bytes()
    }
}

/// Wrapper for an arbitrary sender name.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct SenderName(pub Bytes);

impl From<StaticSenderName> for SenderName {
    fn from(val: StaticSenderName) -> SenderName {
        SenderName(Bytes::from(val))
    }
}

impl From<SenderName> for Bytes {
    fn from(val: SenderName) -> Bytes {
        val.0
    }
}

impl NameIntoBytes for SenderName {
    fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl IdWithNameAndDescription for SenderId {
    type Name = SenderName;

    const DESCRIPTION_MESSAGE_TYPE: MessageTypeId = constants::SENDER_DESCRIPTION;
}

/// Be able to compare `StaticSenderName` and `SenderName`
impl std::cmp::PartialEq<StaticSenderName> for SenderName {
    fn eq(&self, other: &StaticSenderName) -> bool {
        self.0 == Bytes::from_static(other.0)
    }
}

/// Wrapper for a fixed type name, as a static byte array.
///
/// Convertible to `TypeName`
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct StaticMessageTypeName(pub &'static [u8]);

impl From<&'static [u8]> for StaticMessageTypeName {
    fn from(val: &'static [u8]) -> StaticMessageTypeName {
        StaticMessageTypeName(val)
    }
}

impl From<StaticMessageTypeName> for Bytes {
    fn from(val: StaticMessageTypeName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl std::cmp::PartialEq<MessageTypeName> for StaticMessageTypeName {
    fn eq(&self, other: &MessageTypeName) -> bool {
        Bytes::from_static(self.0) == other.0
    }
}

impl NameIntoBytes for StaticMessageTypeName {
    fn into_bytes(self) -> Bytes {
        MessageTypeName::from(self).into_bytes()
    }
}
/// Wrapper for an arbitrary message type name.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct MessageTypeName(pub Bytes);

impl From<&'static [u8]> for MessageTypeName {
    fn from(val: &'static [u8]) -> MessageTypeName {
        MessageTypeName(Bytes::from_static(val))
    }
}

impl From<StaticMessageTypeName> for MessageTypeName {
    fn from(val: StaticMessageTypeName) -> MessageTypeName {
        MessageTypeName(Bytes::from(val))
    }
}

impl From<MessageTypeName> for Bytes {
    fn from(val: MessageTypeName) -> Bytes {
        val.0
    }
}

impl std::cmp::PartialEq<StaticMessageTypeName> for MessageTypeName {
    fn eq(&self, other: &StaticMessageTypeName) -> bool {
        self.0 == Bytes::from_static(other.0)
    }
}
impl NameIntoBytes for MessageTypeName {
    fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl IdWithNameAndDescription for MessageTypeId {
    type Name = SenderName;

    const DESCRIPTION_MESSAGE_TYPE: MessageTypeId = constants::TYPE_DESCRIPTION;
}
