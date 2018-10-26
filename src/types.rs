// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

pub use self::IdToHandle::*;
extern crate bytes;

/// Type wrapped by the various Id types - chosen to match VRPN C++.
pub type IdType = i32;

pub const MAX_VEC_USIZE: usize = (IdType::max_value() - 2) as usize;

pub trait TypeSafeId: Clone + Eq + PartialEq + Ord + PartialOrd {
    fn unwrap(&self) -> IdType;
    fn new(val: IdType) -> Self;
}
pub trait BaseTypeSafeId: TypeSafeId {}

/// Local-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct LocalId<T: TypeSafeId>(pub T);

/// Remote-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct RemoteId<T: TypeSafeId>(pub T);

/// ID for a message type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeId(pub IdType);

/// ID for a sender
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct SenderId(pub IdType);

impl<T: TypeSafeId> TypeSafeId for LocalId<T> {
    fn unwrap(&self) -> IdType {
        self.0.unwrap()
    }
    fn new(val: IdType) -> LocalId<T> {
        LocalId(T::new(val))
    }
}

impl<T: TypeSafeId> TypeSafeId for RemoteId<T> {
    fn unwrap(&self) -> IdType {
        self.0.unwrap()
    }
    fn new(val: IdType) -> RemoteId<T> {
        RemoteId(T::new(val))
    }
}

impl TypeSafeId for TypeId {
    fn unwrap(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> TypeId {
        TypeId(val)
    }
}
impl BaseTypeSafeId for TypeId {}

impl TypeSafeId for SenderId {
    fn unwrap(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> SenderId {
        SenderId(val)
    }
}
impl BaseTypeSafeId for SenderId {}
/*
impl<T> std::cmp::PartialEq for T where T: TypeSafeId {

}
*/

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

bitmask! {
    pub mask ClassOfService : u32 where flags Flags {
        RELIABLE = (1 << 0),
        FIXED_LATENCY = (1 << 1),
        LOW_LATENCY = (1 << 2),
        FIXED_THROUGHPUT = (1 << 3),
        HIGH_THROUGHPUT = (1 << 4),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Default for Version {
    fn default() -> Version {
        Version { major: 0, minor: 0 }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct CookieData {
    pub version: Version,
    pub log_mode: Option<u8>,
}

impl Default for CookieData {
    fn default() -> CookieData {
        CookieData {
            version: Default::default(),
            log_mode: None,
        }
    }
}

impl From<Version> for CookieData {
    fn from(version: Version) -> CookieData {
        CookieData {
            version,
            ..Default::default()
        }
    }
}

impl From<CookieData> for Version {
    fn from(data: CookieData) -> Version {
        data.version
    }
}
