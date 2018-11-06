// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::constants;
use bytes::Bytes;

/// Type wrapped by the various Id types - chosen to match VRPN C++.
pub type IdType = i32;

pub const MAX_VEC_USIZE: usize = (IdType::max_value() - 2) as usize;

pub trait TypeSafeId: Clone + Eq + PartialEq + Ord + PartialOrd {
    fn get(&self) -> IdType;
    fn new(val: IdType) -> Self;
}

pub trait BaseTypeSafeId
where
    Self: TypeSafeId,
    Self::Name: TypedName,
{
    type Name;
    fn name_to_bytes(name: Self::Name) -> Bytes;
    fn description_type() -> TypeId;
}

/// Local-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalId<T: TypeSafeId>(pub T);

/// Remote-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RemoteId<T: TypeSafeId>(pub T);

/// ID for a message type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeId(pub IdType);

/// ID for a sender
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SenderId(pub IdType);

impl<T: TypeSafeId> TypeSafeId for LocalId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> LocalId<T> {
        LocalId(T::new(val))
    }
}

impl<T: TypeSafeId> TypeSafeId for RemoteId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> RemoteId<T> {
        RemoteId(T::new(val))
    }
}

impl TypeSafeId for TypeId {
    fn get(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> TypeId {
        TypeId(val)
    }
}
impl BaseTypeSafeId for TypeId {
    type Name = TypeName;
    fn name_to_bytes(name: Self::Name) -> Bytes {
        Bytes::from_static(name.0)
    }
    fn description_type() -> TypeId {
        constants::TYPE_DESCRIPTION
    }
}

impl TypeSafeId for SenderId {
    fn get(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> SenderId {
        SenderId(val)
    }
}

impl BaseTypeSafeId for SenderId {
    type Name = SenderName;
    fn name_to_bytes(name: Self::Name) -> Bytes {
        Bytes::from_static(name.0)
    }
    fn description_type() -> TypeId {
        constants::SENDER_DESCRIPTION
    }
}

/// Wrapper for an id associated with a handler.
///
/// A bit like Option<T> but the "None" enumerant is called "AnyId" and Some is called SomeId
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum IdToHandle<T> {
    /// Any ID
    AnyId,
    /// One specific ID
    SomeId(T),
}
pub use self::IdToHandle::*;

bitmask! {
    pub mask ClassOfService : u32 where flags ServiceFlags {
        RELIABLE = (1 << 0),
        FIXED_LATENCY = (1 << 1),
        LOW_LATENCY = (1 << 2),
        FIXED_THROUGHPUT = (1 << 3),
        HIGH_THROUGHPUT = (1 << 4),
    }
}

bitmask!{
    pub mask LogMode: u8 where flags LogFlags {
        NONE = 0,
        INCOMING = (1 << 0),
        OUTGOING = (1 << 1),
        INCOMING_OUTGOING = (1 << 0)|(1 << 1)
    }
}

/// @todo temporary
pub type Time = u32;

#[derive(Debug, Clone)]
pub struct HandlerParams {
    pub message_type: TypeId,
    pub sender: SenderId,
    pub msg_time: Time,
    pub buffer: bytes::Bytes,
}

pub trait TypedName {}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct SenderName(pub &'static [u8]);
impl From<SenderName> for Bytes {
    fn from(val: SenderName) -> Bytes {
        Bytes::from_static(val.0)
    }
}
impl TypedName for SenderName {}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TypeName(pub &'static [u8]);
impl From<TypeName> for Bytes {
    fn from(val: TypeName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl TypedName for TypeName {}

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);

pub struct LogDescription {
    /// incoming log file name
    in_name: Bytes,
    /// outgoing log file name
    out_name: Bytes,
}
