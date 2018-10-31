// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use std::error;
use std::fmt;
use types::{BaseTypeSafeId, IdType, LocalId, RemoteId, TypeSafeId};

#[derive(Debug, Clone)]
pub enum TranslationTableError {
    InvalidRemoteId(IdType),
    EmptyEntry,
}

impl fmt::Display for TranslationTableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TranslationTableError::InvalidRemoteId(id) => write!(f, "invalid remote id {}", id),
            TranslationTableError::EmptyEntry => write!(f, "empty entry"),
        }
    }
}

impl error::Error for TranslationTableError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

pub type TranslationResult<T> = Result<T, TranslationTableError>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct TranslationEntry<T: BaseTypeSafeId> {
    pub name: Bytes,
    pub local_id: LocalId<T>,
    pub remote_id: RemoteId<T>,
}

impl<T: BaseTypeSafeId> TranslationEntry<T> {
    pub fn new(name: Bytes, local_id: LocalId<T>, remote_id: RemoteId<T>) -> TranslationEntry<T> {
        TranslationEntry {
            name,
            local_id,
            remote_id,
        }
    }
    pub fn set_local_id(&mut self, local_id: LocalId<T>) {
        self.local_id = local_id;
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TranslationTable<T: BaseTypeSafeId> {
    entries: Vec<Option<TranslationEntry<T>>>,
}

impl<T: BaseTypeSafeId> TranslationTable<T> {
    pub fn new() -> TranslationTable<T> {
        TranslationTable {
            entries: Vec::new(),
        }
    }

    /// Converts a remote ID to the corresponding local ID
    pub fn map_to_local_id(&self, id: RemoteId<T>) -> TranslationResult<Option<LocalId<T>>> {
        let index = id.get();
        if index < 0 {
            return Ok(None);
        }
        if index >= self.entries.len() as i32 {
            return Err(TranslationTableError::InvalidRemoteId(index));
        }
        if let Some(ref entry) = self.entries[index as usize] {
            Ok(Some(entry.local_id.clone()))
        } else {
            Err(TranslationTableError::EmptyEntry)
        }
    }

    pub fn add_remote_entry(
        &mut self,
        name: Bytes,
        remote_id: RemoteId<T>,
        local_id: LocalId<T>,
    ) -> TranslationResult<RemoteId<T>> {
        let real_index = remote_id.get();
        if real_index < 0 {
            return Err(TranslationTableError::InvalidRemoteId(real_index));
        }
        while real_index as usize >= self.entries.len() {
            self.entries.push(None);
        }
        self.entries[real_index as usize] = Some(TranslationEntry {
            name,
            local_id,
            remote_id: remote_id.clone(),
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
                match self.entries[i] {
                    Some(ref mut entry) => entry.set_local_id(local_id),
                    None => {}
                };
                true
            }
            None => false,
        }
    }

    pub fn get_by_local_id(&self, local_id: LocalId<T>) -> Option<&TranslationEntry<T>> {
        let result = self.entries.iter().position(|ref x| match x {
            Some(entry) => entry.local_id == local_id,
            _ => false,
        });
        if result.is_none() {
            return None;
        }
        match result {
            Some(i) => match self.entries[i] {
                Some(ref entry) => Some(&entry),
                None => None,
            },
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn simple() {
        use super::*;
        use types::{RemoteId, SenderId, SenderName};
        let mut table: TranslationTable<SenderId> = TranslationTable::new();
        table
            .add_remote_entry(
                Bytes::from_static(b"asdf"),
                RemoteId(SenderId(0)),
                LocalId(SenderId(0)),
            ).expect("Failed adding remote entry");
    }
}
