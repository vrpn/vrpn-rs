// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::constants;

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
{
    fn description_type() -> TypeId;
}

pub trait BaseTypeSafeIdName<'a>
where
    Self: BaseTypeSafeId,
    Self::Name: TypedName,
{
    type Name;
}

/// Local-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalId<T: BaseTypeSafeId>(pub T);

/// Remote-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RemoteId<T: BaseTypeSafeId>(pub T);

/// ID for a message type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeId(pub IdType);

/// ID for a sender
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SenderId(pub IdType);

impl<T: BaseTypeSafeId> TypeSafeId for LocalId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> LocalId<T> {
        LocalId(T::new(val))
    }
}

impl<T: BaseTypeSafeId> TypeSafeId for RemoteId<T> {
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
    fn description_type() -> TypeId {
        constants::TYPE_DESCRIPTION
    }
}
impl<'a> BaseTypeSafeIdName<'a> for TypeId {
    type Name = TypeName<'a>;
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
    fn description_type() -> TypeId {
        constants::SENDER_DESCRIPTION
    }
}

impl<'a> BaseTypeSafeIdName<'a> for SenderId {
    type Name = SenderName<'a>;
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

pub trait TypedName {
    type Id;
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct SenderName<'a>(pub &'a [u8]);

impl<'a> From<SenderName<'a>> for Bytes {
    fn from(val: SenderName<'a>) -> Bytes {
        Bytes::from(val.0)
    }
}
impl<'a> TypedName for SenderName<'a> {
    type Id = SenderId;
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TypeName<'a>(pub &'a [u8]);
impl<'a> From<TypeName<'a>> for Bytes {
    fn from(val: TypeName<'a>) -> Bytes {
        Bytes::from(val.0)
    }
}

impl<'a> TypedName for TypeName<'a> {
    type Id = TypeId;
}

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);
