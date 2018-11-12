// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::prelude::*;
use crate::{BaseTypeSafeId, Error, LocalId, RemoteId, Result, SenderId, TypeId, TypeSafeId};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry<T: BaseTypeSafeId> {
    name: Bytes,
    local_id: LocalId<T>,
    remote_id: RemoteId<T>,
}

impl<T: BaseTypeSafeId> Entry<T> {
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

    pub fn name(&self) -> &Bytes {
        &self.name
    }
    pub fn local_id(&self) -> LocalId<T> {
        self.local_id
    }
    pub fn remote_id(&self) -> RemoteId<T> {
        self.remote_id
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Table<T: BaseTypeSafeId> {
    entries: Vec<Option<Entry<T>>>,
}

impl<T: BaseTypeSafeId> Default for Table<T> {
    fn default() -> Table<T> {
        Table::new()
    }
}

impl<T: BaseTypeSafeId> Table<T> {
    pub fn new() -> Table<T> {
        Table {
            entries: Vec::new(),
        }
    }

    /// Converts a remote ID to the corresponding local ID
    pub fn map_to_local_id(&self, id: RemoteId<T>) -> Result<Option<LocalId<T>>> {
        let index = id.get();
        if index < 0 {
            return Ok(None);
        }
        if index >= self.entries.len() as i32 {
            return Err(Error::InvalidId(index));
        }
        if let Some(ref entry) = self.entries[index as usize] {
            Ok(Some(entry.local_id))
        } else {
            Err(Error::EmptyEntry)
        }
    }

    pub fn add_remote_entry(
        &mut self,
        name: Bytes,
        remote_id: RemoteId<T>,
        local_id: LocalId<T>,
    ) -> Result<RemoteId<T>> {
        let real_index = remote_id.get();
        if real_index < 0 {
            return Err(Error::InvalidId(real_index));
        }
        while real_index as usize >= self.entries.len() {
            self.entries.push(None);
        }
        self.entries[real_index as usize] = Some(Entry {
            name,
            local_id,
            remote_id,
        });
        Ok(remote_id)
    }

    pub fn add_local_id(&mut self, name: Bytes, local_id: LocalId<T>) -> bool {
        let find_result = self.entries.iter().position(|ref x| match x {
            Some(entry) => entry.name == name,
            _ => false,
        });
        match find_result {
            Some(i) => {
                if let Some(ref mut entry) = self.entries[i] {
                    entry.set_local_id(local_id);
                }
                true
            }
            None => false,
        }
    }

    pub(crate) fn find_by_predicate<F>(&self, f: F) -> Option<&Entry<T>>
    where
        F: Fn(&Entry<T>) -> bool,
    {
        let result = self.entries.iter().position(|ref x| match x {
            Some(entry) => f(entry),
            _ => false,
        });
        match result {
            Some(i) => match self.entries[i] {
                Some(ref entry) => Some(&entry),
                None => None,
            },
            None => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter().flatten()
    }

    pub fn clear(&mut self) {
        self.entries.clear()
    }
}

#[derive(Debug)]
pub struct Tables {
    pub types: Table<TypeId>,
    pub senders: Table<SenderId>,
}

impl Tables {
    pub fn new() -> Tables {
        Tables {
            types: Table::new(),
            senders: Table::new(),
        }
    }

    pub fn clear(&mut self) {
        self.types.clear();
        self.senders.clear();
    }
}

impl Default for Tables {
    fn default() -> Tables {
        Tables::new()
    }
}

/// Trait for type-based dispatching/access of the two translation tables.
///
/// Uniform interface for treating Tables just like the appropriate Table<T>
pub trait MatchingTable<T: BaseTypeSafeId> {
    /// Borrow the correctly-typed translation table
    fn table(&self) -> &Table<T>;
    /// Mutably borrow the correctly-typed translation table
    fn table_mut(&mut self) -> &mut Table<T>;

    /// Convert a remote ID to a local ID, if found.
    fn map_to_local_id(&self, id: RemoteId<T>) -> Result<Option<LocalId<T>>> {
        self.table().map_to_local_id(id)
    }

    /// Record a remote and local ID with the corresponding name.
    fn add_remote_entry(
        &mut self,
        name: Bytes,
        remote_id: RemoteId<T>,
        local_id: LocalId<T>,
    ) -> Result<RemoteId<T>> {
        self.table_mut().add_remote_entry(name, remote_id, local_id)
    }

    /// Gets a shared borrow of an entry, given its local ID.
    fn find_by_local_id(&self, local_id: LocalId<T>) -> Option<&Entry<T>> {
        self.table()
            .find_by_predicate(|entry| entry.local_id() == local_id)
    }

    fn add_local_id(&mut self, name: Bytes, local_id: LocalId<T>) -> bool {
        self.table_mut().add_local_id(name, local_id)
    }
}

impl MatchingTable<SenderId> for Tables {
    fn table(&self) -> &Table<SenderId> {
        &self.senders
    }
    fn table_mut(&mut self) -> &mut Table<SenderId> {
        &mut self.senders
    }
}

impl MatchingTable<TypeId> for Tables {
    fn table(&self) -> &Table<TypeId> {
        &self.types
    }
    fn table_mut(&mut self) -> &mut Table<TypeId> {
        &mut self.types
    }
}

impl<T: BaseTypeSafeId> MatchingTable<T> for Table<T> {
    fn table(&self) -> &Table<T> {
        self
    }
    fn table_mut(&mut self) -> &mut Table<T> {
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        use super::*;
        use crate::types::{RemoteId, SenderId};
        let mut table: Table<SenderId> = Table::new();
        table
            .add_remote_entry(
                Bytes::from_static(b"asdf"),
                RemoteId(SenderId(0)),
                LocalId(SenderId(0)),
            )
            .expect("Failed adding remote entry");
    }
}
