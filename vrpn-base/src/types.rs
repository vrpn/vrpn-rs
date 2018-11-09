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
    Self: TypeSafeId + Clone + Copy + std::fmt::Debug + Eq,
{
    fn description_type() -> TypeId;
}

// pub trait BaseTypeSafeIdName<'a>
// where
//     Self: BaseTypeSafeId,
//     Self::Name: TypedName,
// {
//     type Name;
// }

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

impl TypeId {
    /// Identifies if this is a system message.
    ///
    /// If false, it's a normal (user) message.
    pub fn is_system_message(&self) -> bool {
        self.0 < 0
    }
}
// impl<'a> BaseTypeSafeIdName<'a> for TypeId {
//     type Name = StaticTypeName<'a>;
// }

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

// impl<'a> BaseTypeSafeIdName<'a> for SenderId {
//     type Name = StaticSenderName<'a>;
// }

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

impl<T> IdToHandle<T>
where
    T: PartialEq + Copy,
{
    pub fn matches(&self, other: &T) -> bool {
        match self {
            AnyId => true,
            SomeId(i) => i == other,
        }
    }
}

bitmask! {
    pub mask ClassOfService : u32 where flags ServiceFlags {
        RELIABLE = (1 << 0),
        FIXED_LATENCY = (1 << 1),
        LOW_LATENCY = (1 << 2),
        FIXED_THROUGHPUT = (1 << 3),
        HIGH_THROUGHPUT = (1 << 4),
    }
}

// pub trait TypedName {
//     type Id;
// }

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct StaticSenderName(pub &'static [u8]);

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct SenderName(pub Bytes);

impl From<StaticSenderName> for SenderName {
    fn from(val: StaticSenderName) -> SenderName {
        SenderName(Bytes::from(val))
    }
}

impl From<StaticSenderName> for Bytes {
    fn from(val: StaticSenderName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl From<SenderName> for Bytes {
    fn from(val: SenderName) -> Bytes {
        val.0
    }
}

impl std::cmp::PartialEq<SenderName> for StaticSenderName {
    fn eq(&self, other: &SenderName) -> bool {
        Bytes::from_static(self.0) == other.0
    }
}

impl std::cmp::PartialEq<StaticSenderName> for SenderName {
    fn eq(&self, other: &StaticSenderName) -> bool {
        self.0 == Bytes::from_static(other.0)
    }
}

// impl TypedName for StaticSenderName {
//     type Id = SenderId;
// }

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct StaticTypeName(pub &'static [u8]);

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TypeName(pub Bytes);

impl From<StaticTypeName> for TypeName {
    fn from(val: StaticTypeName) -> TypeName {
        TypeName(Bytes::from(val))
    }
}

impl From<StaticTypeName> for Bytes {
    fn from(val: StaticTypeName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl From<TypeName> for Bytes {
    fn from(val: TypeName) -> Bytes {
        val.0
    }
}

impl std::cmp::PartialEq<TypeName> for StaticTypeName {
    fn eq(&self, other: &TypeName) -> bool {
        Bytes::from_static(self.0) == other.0
    }
}

impl std::cmp::PartialEq<StaticTypeName> for TypeName {
    fn eq(&self, other: &StaticTypeName) -> bool {
        self.0 == Bytes::from_static(other.0)
    }
}

// impl TypedName for StaticTypeName {
//     type Id = TypeId;
// }

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);
