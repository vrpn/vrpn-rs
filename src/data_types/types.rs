// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Basic types used across VRPN.
//!
//! Mostly related to sender and type IDs and names,
//! plus the math types `Vec3` and `Quat`

use crate::constants;
use bytes::Bytes;
use cgmath::Vector3;

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

/// A named ID, extending `BaseTypeSafeID`
pub trait BaseTypeSafeIdName
where
    Self::Name: Into<Bytes>,
{
    type Name;
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

bitflags! {
    /// Class of service flags matching those in the original vrpn
    pub struct ClassOfService : u32 {
        /// Results in TCP transport if available
        const RELIABLE = (1 << 0);
        const FIXED_LATENCY = (1 << 1);
        /// Results in UDP transport if available
        const LOW_LATENCY = (1 << 2);
        const FIXED_THROUGHPUT = (1 << 3);
        const HIGH_THROUGHPUT = (1 << 4);
    }
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
pub struct StaticTypeName(pub &'static [u8]);

impl From<&'static [u8]> for StaticTypeName {
    fn from(val: &'static [u8]) -> StaticTypeName {
        StaticTypeName(val)
    }
}

impl From<StaticTypeName> for Bytes {
    fn from(val: StaticTypeName) -> Bytes {
        Bytes::from_static(val.0)
    }
}

impl std::cmp::PartialEq<TypeName> for StaticTypeName {
    fn eq(&self, other: &TypeName) -> bool {
        Bytes::from_static(self.0) == other.0
    }
}

/// Wrapper for an arbitrary type name.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TypeName(pub Bytes);

impl From<&'static [u8]> for TypeName {
    fn from(val: &'static [u8]) -> TypeName {
        TypeName(Bytes::from_static(val))
    }
}

impl From<StaticTypeName> for TypeName {
    fn from(val: StaticTypeName) -> TypeName {
        TypeName(Bytes::from(val))
    }
}

impl From<TypeName> for Bytes {
    fn from(val: TypeName) -> Bytes {
        val.0
    }
}

impl std::cmp::PartialEq<StaticTypeName> for TypeName {
    fn eq(&self, other: &StaticTypeName) -> bool {
        self.0 == Bytes::from_static(other.0)
    }
}

/// Sequence number - not used on receive side, only used for sniffers (?)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SequenceNumber(pub u32);

/// Sensor ID for trackers.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Sensor(pub i32);

/// A 3D vector of 64-bit floats
pub type Vec3 = Vector3<f64>;

/// A (typically unit) quaternion corresponding to a rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub s: f64,
    pub v: Vec3,
}

impl Quat {
    /// Create from scalar part and vector part.
    pub fn from_sv(s: f64, v: Vec3) -> Quat {
        Quat { s, v }
    }

    /// Create from all four coefficients: mind the order!
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Quat {
        Quat {
            s: w,
            v: Vec3::new(x, y, z),
        }
    }

    /// Return an identity rotation
    pub fn identity() -> Quat {
        Quat {
            s: 1.0,
            v: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl From<cgmath::Quaternion<f64>> for Quat {
    fn from(q: cgmath::Quaternion<f64>) -> Self {
        Quat { s: q.s, v: q.v }
    }
}

impl From<Quat> for cgmath::Quaternion<f64> {
    fn from(q: Quat) -> Self {
        cgmath::Quaternion::from_sv(q.s, q.v)
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
    let raw = id.get();
    if raw < 0 {
        RangedId::BelowZero(raw)
    } else {
        let index = raw as usize;
        if index < len {
            RangedId::InArray(raw)
        } else {
            RangedId::AboveArray(raw)
        }
    }
}
