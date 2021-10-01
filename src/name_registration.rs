// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>
use crate::{
    data_types::{
        id_types::{determine_id_range, IdType, LocalId, RangedId, UnwrappedId, MAX_VEC_USIZE},
        name_types::NameIntoBytes,
        IdWithNameAndDescription, MessageTypeId,
    },
    Result, VrpnError,
};
use bytes::Bytes;
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Name(Bytes);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) struct MessageTypeIndex(usize);

pub(crate) trait TryIntoIndex {
    type Index;
    fn try_into_index(self, len: usize) -> Result<Self::Index>;
}

impl TryIntoIndex for MessageTypeId {
    type Index = MessageTypeIndex;
    fn try_into_index(self, len: usize) -> Result<Self::Index> {
        use RangedId::*;
        match determine_id_range(self, len) {
            BelowZero(v) => Err(VrpnError::InvalidId(v)),
            AboveArray(v) => Err(VrpnError::InvalidId(v)),
            InArray(index) => Ok(MessageTypeIndex(index as usize)),
        }
    }
}

// impl TryIntoIndex for SenderId {
//     fn try_into_index(self, len: usize) -> Result<Index> {
//         use RangedId::*;
//         match determine_id_range(self, len) {
//             BelowZero(v) => Err(VrpnError::InvalidId(v)),
//             AboveArray(v) => Err(VrpnError::InvalidId(v)),
//             InArray(index) => Ok(Index(index as usize)),
//         }
//     }
// }

pub(crate) trait RegisterableId:
    UnwrappedId + TryIntoIndex + IdWithNameAndDescription
where
    Self::Name: NameIntoBytes,
{
}
impl<I: UnwrappedId + TryIntoIndex + IdWithNameAndDescription> RegisterableId for I {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum InsertOrGet<I> {
    /// This was an existing mapping with the given ID
    Found(I),
    /// This was a new mapping, which has been registered and received the given ID
    New(I),
}

impl<I: Copy> InsertOrGet<I> {
    /// Access the wrapped ID, no matter if it was new or not.
    pub fn into_inner(&self) -> I {
        match self {
            InsertOrGet::Found(v) => *v,
            InsertOrGet::New(v) => *v,
        }
    }
}
pub(crate) struct NameRegistration<I: RegisterableId> {
    /// Index is the local type ID
    names: Vec<Name>,
    ids_by_name: HashMap<Name, LocalId<I>>,
}

impl<I: RegisterableId> Default for NameRegistration<I> {
    fn default() -> NameRegistration<I> {
        NameRegistration {
            names: vec![],
            ids_by_name: HashMap::default(),
        }
    }
}

impl<I: RegisterableId> NameRegistration<I> {
    fn try_insert(&mut self, name: &Name) -> Result<LocalId<I>> {
        if self.names.len() > MAX_VEC_USIZE {
            return Err(VrpnError::TooManyMappings);
        }
        self.names.push(name.clone());
        let id = LocalId(I::new((self.names.len() - 1) as IdType));
        self.ids_by_name.insert(name.clone(), id);
        Ok(id)
    }
    /// Get the ID based on the name, if it's registered, otherwise insert it if there's room.
    pub(crate) fn try_insert_or_get(
        &mut self,
        name: impl Into<I::Name>,
    ) -> Result<InsertOrGet<LocalId<I>>> {
        let name: Bytes = name.into().into_bytes();
        let name = Name(name);
        Ok(match self.ids_by_name.get(&name) {
            Some(id) => InsertOrGet::Found(*id),
            None => InsertOrGet::New(self.try_insert(&name)?),
        })
    }

    /// Get the ID based on the name, if it's registered.
    pub(crate) fn try_get_by_name(&self, name: impl Into<I::Name>) -> Option<LocalId<I>> {
        let name: Bytes = name.into().into_bytes();
        let name = Name(name);
        self.ids_by_name.get(&name).copied()
    }
}
