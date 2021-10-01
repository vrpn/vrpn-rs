// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Code for associating names and local IDs with their remote equivalents.

use std::convert::TryFrom;

use crate::{
    data_types::{id_types::*, GenericMessage},
    type_dispatcher::IntoDescriptionMessage,
    Result, VrpnError,
};
use bytes::Bytes;

/// An entry in a translation table
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Entry<T: UnwrappedId> {
    name: Bytes,
    local_id: LocalId<T>,
    remote_id: RemoteId<T>,
}

impl<T: UnwrappedId> Entry<T> {
    fn new(name: Bytes, local_id: LocalId<T>, remote_id: RemoteId<T>) -> Entry<T> {
        Entry {
            name,
            local_id,
            remote_id,
        }
    }
    fn set_local_id(&mut self, local_id: LocalId<T>) {
        self.local_id = local_id;
    }

    pub(crate) fn name(&self) -> &Bytes {
        &self.name
    }
    pub(crate) fn local_id(&self) -> LocalId<T> {
        self.local_id
    }
    pub(crate) fn remote_id(&self) -> RemoteId<T> {
        self.remote_id
    }
}

impl<T: IntoDescriptionMessage + UnwrappedId> TryFrom<Entry<T>> for GenericMessage {
    type Error = VrpnError;

    fn try_from(value: Entry<T>) -> std::result::Result<Self, Self::Error> {
        value.local_id.into_description_message(value.name)
    }
}

/// A structure mapping names and local IDs to their remote equivalents
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TranslationTable<T: UnwrappedId> {
    entries: Vec<Option<Entry<T>>>,
}

impl<T: UnwrappedId> Default for TranslationTable<T> {
    fn default() -> TranslationTable<T> {
        TranslationTable::new()
    }
}

impl<T: UnwrappedId> TranslationTable<T> {
    /// Create a translation table
    pub fn new() -> TranslationTable<T> {
        TranslationTable {
            entries: Vec::new(),
        }
    }

    fn determine_remote_id_range(&self, id: RemoteId<T>) -> RangedId {
        determine_id_range(id.into_id(), self.entries.len())
    }

    /// Converts a remote ID to the corresponding local ID
    pub(crate) fn map_to_local_id(&self, id: RemoteId<T>) -> Result<Option<LocalId<T>>> {
        use RangedId::*;
        match self.determine_remote_id_range(id) {
            BelowZero(_) => Ok(None),
            AboveArray(v) => Err(VrpnError::InvalidId(v)),
            InArray(v) => match &self.entries[v as usize] {
                Some(entry) => Ok(Some(entry.local_id)),
                None => Err(VrpnError::EmptyEntry),
            },
        }
    }

    pub(crate) fn add_remote_entry(
        &mut self,
        name: Bytes,
        remote_id: RemoteId<T>,
        local_id: LocalId<T>,
    ) -> Result<RemoteId<T>> {
        use RangedId::*;
        let index = match self.determine_remote_id_range(remote_id) {
            BelowZero(v) => return Err(VrpnError::InvalidId(v)),
            AboveArray(v) => {
                self.entries.resize(v as usize + 1, None);
                v as usize
            }
            InArray(v) => v as usize,
        };
        let new_entry = Entry::new(name, local_id, remote_id);
        self.entries[index] = Some(new_entry);
        Ok(remote_id)
    }

    /// Adds a local ID to a name that was already in the table.
    /// Returns true if the name has been found, false if not found.
    pub(crate) fn add_local_id(&mut self, name: Bytes, local_id: LocalId<T>) -> bool {
        let find_result = self.entries.iter().position(|ref x| match x {
            Some(entry) => entry.name == name,
            _ => false,
        });
        match find_result {
            Some(i) => {
                if let Some(entry) = &mut self.entries[i] {
                    entry.set_local_id(local_id);
                }
                true
            }
            None => false,
        }
    }

    /// Attempts to find an entry satisfying your predicate,
    /// returning a shared borrow of it if found.
    pub(crate) fn find_by_predicate<F>(&self, f: F) -> Option<&Entry<T>>
    where
        F: Fn(&Entry<T>) -> bool,
    {
        let result = self.entries.iter().position(|x| match x {
            Some(entry) => f(entry),
            _ => false,
        });
        match result {
            Some(i) => match &self.entries[i] {
                Some(entry) => Some(entry),
                None => None,
            },
            None => None,
        }
    }

    /// Get an iterator to non-None table entries.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter().flatten()
    }

    /// Deletes every entry in the table
    pub fn clear(&mut self) {
        self.entries.clear()
    }
}

/// A type owning two translation tables: one for message types, one for senders.
#[derive(Debug)]
pub struct TranslationTables {
    types: TranslationTable<MessageTypeId>,
    senders: TranslationTable<SenderId>,
}

impl TranslationTables {
    pub fn new() -> TranslationTables {
        TranslationTables {
            types: TranslationTable::new(),
            senders: TranslationTable::new(),
        }
    }

    pub fn clear(&mut self) {
        self.types.clear();
        self.senders.clear();
    }
}

impl Default for TranslationTables {
    fn default() -> TranslationTables {
        TranslationTables::new()
    }
}

impl AsRef<TranslationTable<SenderId>> for TranslationTables {
    fn as_ref(&self) -> &TranslationTable<SenderId> {
        &self.senders
    }
}

impl AsMut<TranslationTable<SenderId>> for TranslationTables {
    fn as_mut(&mut self) -> &mut TranslationTable<SenderId> {
        &mut self.senders
    }
}

impl AsRef<TranslationTable<MessageTypeId>> for TranslationTables {
    fn as_ref(&self) -> &TranslationTable<MessageTypeId> {
        &self.types
    }
}

impl AsMut<TranslationTable<MessageTypeId>> for TranslationTables {
    fn as_mut(&mut self) -> &mut TranslationTable<MessageTypeId> {
        &mut self.types
    }
}

pub(crate) trait InTranslationTables: UnwrappedId
where
    TranslationTables: AsRef<TranslationTable<Self>>,
{
}

impl<T: UnwrappedId> InTranslationTables for T where TranslationTables: AsRef<TranslationTable<Self>>
{}

pub(crate) trait InMutTranslationTables: UnwrappedId
where
    TranslationTables: AsMut<TranslationTable<Self>>,
{
}

impl<T: UnwrappedId> InMutTranslationTables for T where
    TranslationTables: AsMut<TranslationTable<Self>>
{
}

pub(crate) trait TranslationTableExt<I: UnwrappedId>: AsRef<TranslationTable<I>> {
    /// Gets a shared borrow of an entry, given its local ID.
    fn find_by_local_id(&self, local_id: LocalId<I>) -> Option<&Entry<I>> {
        (self.as_ref() as &TranslationTable<I>)
            .find_by_predicate(|entry| entry.local_id() == local_id)
    }
}

impl<T: AsRef<TranslationTable<I>>, I: UnwrappedId> TranslationTableExt<I> for T {}

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        use super::*;
        use crate::data_types::id_types::{RemoteId, SenderId};
        let mut table: TranslationTable<SenderId> = TranslationTable::new();
        table
            .add_remote_entry(
                Bytes::from_static(b"asdf"),
                RemoteId(SenderId(0)),
                LocalId(SenderId(0)),
            )
            .expect("Failed adding remote entry");
    }
}
