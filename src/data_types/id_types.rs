// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Basic ID types used across VRPN.

use crate::buffer_unbuffer::WrappedConstantSize;

/// Type wrapped by the various Id types - chosen to match VRPN C++.
pub type IdType = i32;

pub const MAX_VEC_USIZE: usize = (IdType::max_value() - 2) as usize;

/// Trait for types that wrap an integer to treat it as an ID, namely `MessageTypeId` and `SenderId`
///
/// Provides easy, uniform construction and retrieval.
pub trait Id: Copy + Clone + Eq + PartialEq + Ord + PartialOrd {
    fn get(&self) -> i32;
    fn new(val: i32) -> Self;
}

/// Trait for things that can be converted into an ID.
///
/// Implemented for all types satisfying `UnwrappedId` (so, `MessageTypeId` and `SenderId`)
/// as well as the `LocalId<T>` and `RemoteId<T>` wrappers.
pub trait IntoId: Id {
    /// Base ID type. Self in the case of UnwrappedId, otherwise the thing that's being wrapped.
    type BaseId: Id;
    fn into_id(self) -> Self::BaseId;
}

/// Trait for only (`MessageTypeId` and `SenderId`)
pub trait UnwrappedId: Id {}

impl<T: UnwrappedId> IntoId for T {
    type BaseId = T;

    fn into_id(self) -> Self::BaseId {
        self
    }
}

/// Local-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalId<T: UnwrappedId>(pub T);

/// Implement `Id` for all LocalId types wrapping a `UnwrappedId`
impl<T: UnwrappedId> Id for LocalId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> LocalId<T> {
        LocalId(T::new(val))
    }
}

/// Implement `IntoId` by unwrapping the ID type.
impl<T: UnwrappedId> IntoId for LocalId<T> {
    type BaseId = T;
    fn into_id(self) -> Self::BaseId {
        self.0
    }
}

/// Remote-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RemoteId<T: UnwrappedId>(pub T);

impl<T: UnwrappedId> IntoId for RemoteId<T> {
    type BaseId = T;
    fn into_id(self) -> Self::BaseId {
        self.0
    }
}

impl<T: UnwrappedId> Id for RemoteId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> RemoteId<T> {
        RemoteId(T::new(val))
    }
}

/// ID for a message type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageTypeId(pub IdType);

impl MessageTypeId {
    /// Identifies if this is a system message.
    ///
    /// If false, it's a normal (user) message.
    pub fn is_system_message(&self) -> bool {
        self.0 < 0
    }
}

impl Id for MessageTypeId {
    fn get(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> MessageTypeId {
        MessageTypeId(val)
    }
}

impl UnwrappedId for MessageTypeId {}

impl WrappedConstantSize for MessageTypeId {
    type WrappedType = IdType;
    fn get(&self) -> Self::WrappedType {
        Id::get(self)
    }
    fn new(v: Self::WrappedType) -> Self {
        MessageTypeId(v)
    }
}

/// ID for a sender
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SenderId(pub IdType);

impl Id for SenderId {
    fn get(&self) -> IdType {
        self.0
    }
    fn new(val: IdType) -> SenderId {
        SenderId(val)
    }
}

impl UnwrappedId for SenderId {}

impl WrappedConstantSize for SenderId {
    type WrappedType = IdType;
    fn get(&self) -> Self::WrappedType {
        Id::get(self)
    }
    fn new(v: Self::WrappedType) -> Self {
        SenderId(v)
    }
}

/// Check for a match against an optional filter.
///
/// If the filter is `None`, it always matches.
///
/// ```
/// use vrpn::data_types::id_types::*;
/// let id = SenderId::new(5);
/// assert!(id_filter_matches(None, id));
/// assert!(id_filter_matches(Some(SenderId::new(5)), id));
/// assert!(!id_filter_matches(Some(SenderId::new(3)), id));
/// ```
pub fn id_filter_matches<T>(filter: Option<T>, other: T) -> bool
where
    T: Id,
{
    match filter {
        None => true,
        Some(i) => i == other,
    }
}

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);

impl WrappedConstantSize for SequenceNumber {
    type WrappedType = u32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        SequenceNumber(v)
    }
}

/// Sensor ID for trackers.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Sensor(pub i32);

impl WrappedConstantSize for Sensor {
    type WrappedType = i32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Sensor(v)
    }
}

pub(crate) enum RangedId {
    BelowZero(IdType),
    InArray(IdType),
    AboveArray(IdType),
}

/// Categorize an ID into either below array, in array, or above array.
///
/// Typically, calling code will then match on the result and make one or more
/// of the variants produce an error. However, which ones are errors vary between
/// functions.
pub(crate) fn determine_id_range<T: UnwrappedId>(id: T, len: usize) -> RangedId {
    let id = id.get();
    if id < 0 {
        RangedId::BelowZero(id)
    } else if (id as usize) < len {
        RangedId::InArray(id)
    } else {
        RangedId::AboveArray(id)
    }
}
