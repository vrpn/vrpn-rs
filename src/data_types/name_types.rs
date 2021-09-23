// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Name types used across VRPN

use bytes::Bytes;

/// A named ID, extending `BaseTypeSafeID`
pub trait BaseTypeSafeIdName
where
    Self::Name: Into<Bytes>,
{
    type Name;
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
