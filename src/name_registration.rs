// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>
use crate::{
    data_types::{
        id_types::{
            categorize_id, CategorizedId, Id, IdType, IdTypeUnsigned, LocalId, UnwrappedId,
            MAX_VEC_USIZE,
        },
        name_types::NameIntoBytes,
        IdWithNameAndDescription,
    },
    Result, VrpnError,
};
use bytes::Bytes;
use std::{collections::HashMap, convert::TryInto};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct Name(Bytes);

impl AsRef<Bytes> for Name {
    fn as_ref(&self) -> &Bytes {
        &self.0
    }
}
pub(crate) trait RegisterableId: UnwrappedId + IdWithNameAndDescription
where
    Self::Name: NameIntoBytes,
{
}
impl<I: UnwrappedId + IdWithNameAndDescription> RegisterableId for I {}

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

pub(crate) trait IntoCorrespondingName<I: IdWithNameAndDescription>: Into<I::Name> {
    fn into_corresponding_name(self) -> I::Name;
}
impl<N, I> IntoCorrespondingName<I> for N
where
    N: Into<I::Name>,
    I: IdWithNameAndDescription,
{
    fn into_corresponding_name(self) -> I::Name {
        self.into()
    }
}

/// Interface for registering/associating names with IDs.
pub(crate) trait LocalNameRegistration {
    type IdType: IdWithNameAndDescription;
    type CategorizedId;

    /// Get the ID based on the name, if it's registered, otherwise insert it if there's room.
    fn try_insert_or_get<N>(&mut self, name: N) -> Result<InsertOrGet<LocalId<Self::IdType>>>
    where
        N: IntoCorrespondingName<Self::IdType>;

    /// Get the ID based on the name, if it's registered.
    fn try_get_id_by_name<N>(&self, name: N) -> Option<LocalId<Self::IdType>>
    where
        N: IntoCorrespondingName<Self::IdType>;

    /// Categorize a given ID based on whether or not it is valid
    fn categorize_id(&self, id: Self::IdType) -> Self::CategorizedId;
}

pub(crate) trait IterableNameRegistration<'a>: LocalNameRegistration {
    type Iterator: Iterator<Item = (LocalId<Self::IdType>, &'a Name)>;
    fn iter(&'a self) -> Self::Iterator;
}

pub(crate) trait ExtraDataById: LocalNameRegistration {
    type ExtraData;
    fn try_get_data(&self, id: Self::IdType) -> Result<&Self::ExtraData>;

    fn try_get_data_mut(&mut self, id: Self::IdType) -> Result<&mut Self::ExtraData>;
}

/// A container that implements LocalNameRegistration minimally
#[derive(Debug, Clone)]
pub(crate) struct NameRegistrationContainer<I: RegisterableId> {
    /// Index is the local type ID
    names: Vec<Name>,
    ids_by_name: HashMap<Name, LocalId<I>>,
}

impl<I: RegisterableId> Default for NameRegistrationContainer<I> {
    fn default() -> NameRegistrationContainer<I> {
        NameRegistrationContainer {
            names: vec![],
            ids_by_name: HashMap::default(),
        }
    }
}

impl<I: IdWithNameAndDescription> LocalNameRegistration for NameRegistrationContainer<I> {
    type IdType = I;
    type CategorizedId = CategorizedId;

    fn try_insert_or_get<N: IntoCorrespondingName<I>>(
        &mut self,
        name: N,
    ) -> Result<InsertOrGet<LocalId<I>>> {
        let name: I::Name = name.into_corresponding_name();
        let name = name.into_bytes();
        let name = Name(name);
        Ok(match self.ids_by_name.get(&name) {
            Some(id) => InsertOrGet::Found(*id),
            None => InsertOrGet::New(self.try_insert(&name)?),
        })
    }

    fn try_get_id_by_name<N: IntoCorrespondingName<I>>(&self, name: N) -> Option<LocalId<I>> {
        let name: Bytes = name.into().into_bytes();
        let name = Name(name);
        self.ids_by_name.get(&name).copied()
    }

    fn categorize_id(&self, id: Self::IdType) -> Self::CategorizedId {
        categorize_id(id, self.names.len())
    }
}

impl<I: RegisterableId> NameRegistrationContainer<I> {
    fn try_insert(&mut self, name: &Name) -> Result<LocalId<I>> {
        if self.names.len() > MAX_VEC_USIZE {
            return Err(VrpnError::TooManyMappings);
        }
        self.names.push(name.clone());
        let id = LocalId(I::new((self.names.len() - 1) as IdType));
        self.ids_by_name.insert(name.clone(), id);
        Ok(id)
    }
}

pub(crate) struct NameRegIter<'a, I: RegisterableId> {
    container: &'a NameRegistrationContainer<I>,
    i: IdTypeUnsigned,
}
impl<'a, I: RegisterableId> NameRegIter<'a, I> {
    fn new(container: &'a NameRegistrationContainer<I>) -> Self {
        Self { container, i: 0 }
    }
}
impl<'a, I: RegisterableId> Iterator for NameRegIter<'a, I> {
    type Item = (LocalId<I>, &'a Name);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.i;
        match self.container.names.get(id as usize) {
            Some(name) => {
                self.i += 1;
                Some((LocalId(I::new(id.try_into().unwrap())), name))
            }
            None => None,
        }
    }
}
impl<'a, I: 'a + RegisterableId> IterableNameRegistration<'a> for NameRegistrationContainer<I> {
    type Iterator = NameRegIter<'a, I>;

    fn iter(&'a self) -> Self::Iterator {
        NameRegIter::new(self)
    }
}

/// Container that wraps a type implementing LocalNameRegistration, storing one value U per ID.
#[derive(Debug)]
pub(crate) struct PerIdData<T: LocalNameRegistration, U: std::fmt::Debug> {
    inner: T,
    data: Vec<U>,
}

impl<T: LocalNameRegistration, U: std::fmt::Debug> PerIdData<T, U> {
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            data: vec![],
        }
    }

    fn try_get_data_impl(&self, id: T::IdType) -> Result<&U> {
        let id = id.get();
        let index: usize = id.try_into().map_err(|_| VrpnError::InvalidId(id))?;
        self.data.get(index).ok_or(VrpnError::InvalidId(id))
    }

    fn try_get_data_mut_impl(&mut self, id: T::IdType) -> Result<&mut U> {
        let id = id.get();

        let index: usize = id.try_into().map_err(|_| VrpnError::InvalidId(id))?;
        let x = &self.data[index];
        self.data.get_mut(index).ok_or(VrpnError::InvalidId(id))
    }
}

// Implemented so we can intercept insertion.
impl<T: LocalNameRegistration<CategorizedId = CategorizedId>, U: std::fmt::Debug + Default>
    LocalNameRegistration for PerIdData<T, U>
{
    type IdType = T::IdType;

    fn try_insert_or_get<N>(&mut self, name: N) -> Result<InsertOrGet<LocalId<Self::IdType>>>
    where
        N: IntoCorrespondingName<Self::IdType>,
    {
        let name = name.into_corresponding_name();
        Ok(match self.inner.try_insert_or_get(name.clone())? {
            InsertOrGet::Found(id) => InsertOrGet::Found(id),
            InsertOrGet::New(id) => {
                let index: usize = id.get().try_into().unwrap();
                assert_eq!(self.data.len(), index);
                self.data.push(U::default());
                InsertOrGet::New(id)
            }
        })
    }

    fn try_get_id_by_name<N>(&self, name: N) -> Option<LocalId<Self::IdType>>
    where
        N: IntoCorrespondingName<Self::IdType>,
    {
        self.inner.try_get_id_by_name(name)
    }

    type CategorizedId = T::CategorizedId;

    fn categorize_id(&self, id: Self::IdType) -> Self::CategorizedId {
        self.inner.categorize_id(id)
    }
}

impl<T: LocalNameRegistration<CategorizedId = CategorizedId>, U: std::fmt::Debug + Default>
    ExtraDataById for PerIdData<T, U>
{
    type ExtraData = U;

    fn try_get_data(&self, id: Self::IdType) -> Result<&Self::ExtraData> {
        self.try_get_data_impl(id)
    }

    fn try_get_data_mut(&mut self, id: Self::IdType) -> Result<&mut Self::ExtraData> {
        self.try_get_data_mut_impl(id)
    }
}

impl<T: LocalNameRegistration<CategorizedId = CategorizedId>, U: std::fmt::Debug + Default> AsRef<T>
    for PerIdData<T, U>
{
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
