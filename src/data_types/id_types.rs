// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Basic ID types used across VRPN.

use crate::buffer_unbuffer::WrappedConstantSize;

use super::{
    constants,
    name_types::{BaseTypeSafeIdName, SenderName, TypeName},
};

/// Type wrapped by the various Id types - chosen to match VRPN C++.
pub type IdType = i32;

pub const MAX_VEC_USIZE: usize = (IdType::max_value() - 2) as usize;

/// Trait for types that wrap an integer to treat it as an ID, namely `TypeId` and `SenderId`
///
/// Provides easy, uniform construction and retrieval.
pub trait TypeSafeId: Copy + Clone + Eq + PartialEq + Ord + PartialOrd {
    fn get(&self) -> IdType;
    fn new(val: IdType) -> Self;
}

/// Trait for things that can be converted into an ID.
///
/// Implemented for all types satisfying `BaseTypeSafeId` (so, `TypeId` and `SenderId`)
/// as well as the `LocalId<T>` and `RemoteId<T>` wrappers.
pub trait IntoId: TypeSafeId {
    /// Base ID type. Self in the case of BaseTypeSafeId, otherwise the thing that's being wrapped.
    type BaseId: BaseTypeSafeId;
    fn into_id(self) -> Self::BaseId;
}

/// Trait for the base type safe ID, extending `TypeSafeId`.
///
/// This is implemented only by `TypeId` and `SenderId`, not their local/remote wrappers.
pub trait BaseTypeSafeId:
    TypeSafeId + Clone + Copy + std::fmt::Debug + PartialEq + Eq + BaseTypeSafeIdName
{
    fn description_type() -> TypeId;
}

/// All `BaseTypeSafeId` are a TypeSafeId so conversion is trivial.
impl<T: BaseTypeSafeId> IntoId for T {
    type BaseId = T;
    fn into_id(self) -> Self::BaseId {
        self
    }
}

/// Local-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalId<T: BaseTypeSafeId>(pub T);

/// Implement `TypeSafeId` for all LocalId types wrapping a `BaseTypeSafeId`
impl<T: BaseTypeSafeId> TypeSafeId for LocalId<T> {
    fn get(&self) -> IdType {
        self.0.get()
    }
    fn new(val: IdType) -> LocalId<T> {
        LocalId(T::new(val))
    }
}

/// Implement `IntoId` by unwrapping the ID type.
impl<T: BaseTypeSafeId> IntoId for LocalId<T> {
    type BaseId = T;
    fn into_id(self) -> Self::BaseId {
        self.0
    }
}

/// Remote-side ID in the translation table
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RemoteId<T: BaseTypeSafeId>(pub T);

impl<T: BaseTypeSafeId> IntoId for RemoteId<T> {
    type BaseId = T;
    fn into_id(self) -> Self::BaseId {
        self.0
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

/// ID for a message type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeId(pub IdType);

impl TypeId {
    /// Identifies if this is a system message.
    ///
    /// If false, it's a normal (user) message.
    pub fn is_system_message(&self) -> bool {
        self.0 < 0
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

impl BaseTypeSafeIdName for TypeId {
    type Name = TypeName;
}

/// ID for a sender
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SenderId(pub IdType);

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

impl BaseTypeSafeIdName for SenderId {
    type Name = SenderName;
}

/// Check for a match against an optional filter.
///
/// If the filter is `None`, it always matches.
///
/// ```
/// use vrpn::types::*;
/// let id = SenderId::new(5);
/// assert!(id_filter_matches(None, id));
/// assert!(id_filter_matches(Some(SenderId::new(5)), id));
/// assert!(!id_filter_matches(Some(SenderId::new(3)), id));
/// ```
pub fn id_filter_matches<T>(filter: Option<T>, other: T) -> bool
where
    T: TypeSafeId,
{
    match filter {
        None => true,
        Some(i) => i == other,
    }
}

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);

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
pub(crate) fn determine_id_range<T: BaseTypeSafeId>(id: T, len: usize) -> RangedId {
    let id = id.get();
    if id < 0 {
        RangedId::BelowZero(id)
    } else {
        let id = id as usize;
        if id < len {
            RangedId::InArray(id)
        } else {
            RangedId::AboveArray(id)
        }
    }
}
