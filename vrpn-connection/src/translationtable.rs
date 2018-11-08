// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{typedispatcher, Error, Result};
use vrpn_base::{
    message::{Description, SequencedMessage},
    types::{BaseTypeSafeId, IdType, LocalId, RemoteId, TypeSafeId},
};
use vrpn_buffer::{buffer, Buffer};

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

    pub fn buffer_description_ref(&self, buf: &mut BytesMut) -> Result<()> {
        // let LocalId(id) = self.local_id.clone();
        // let msg = SequencedMessage::from(Description::new(id, self.name.clone()));
        // buf.reserve(msg.required_buffer_size());
        // msg.buffer_ref(buf)
        //     .map_err(|e| Error::BufferError(e))
        unimplemented!();
    }

    pub fn pack_description(&self) -> Result<Bytes> {
        let mut buf = BytesMut::new();
        self.buffer_description_ref(&mut buf)?;
        Ok(buf.freeze())
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
    pub fn map_to_local_id(&self, id: RemoteId<T>) -> Result<Option<LocalId<T>>> {
        let index = id.get();
        if index < 0 {
            return Ok(None);
        }
        if index >= self.entries.len() as i32 {
            return Err(Error::InvalidRemoteId(index));
        }
        if let Some(ref entry) = self.entries[index as usize] {
            Ok(Some(entry.local_id.clone()))
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
            return Err(Error::InvalidRemoteId(real_index));
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

    pub fn handle_description_message(&mut self, desc: Description<T>) {
        // let local_id = match self.find_by_name(desc.name) {
        //     Some(entry) => entry.local_id,
        //     None => entry.local_id
        // };
    }

    fn find_by_predicate<F>(&self, f: F) -> Option<&TranslationEntry<T>>
    where
        F: Fn(&TranslationEntry<T>) -> bool,
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

    pub fn find_by_name(&self, name: Bytes) -> Option<&TranslationEntry<T>> {
        self.find_by_predicate(|entry| entry.name == name)
    }

    pub fn find_by_local_id(&self, local_id: LocalId<T>) -> Option<&TranslationEntry<T>> {
        self.find_by_predicate(|entry| entry.local_id == local_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TranslationEntry<T>> {
        self.entries.iter().flatten()
    }

    pub fn buffer_descriptions_ref(&self, buf: &mut BytesMut) -> Result<()> {
        for entry in self.entries.iter().flatten() {
            entry.buffer_description_ref(buf)?;
        }
        Ok(())
    }
    pub fn buffer_descriptions(&self) -> Result<Bytes> {
        let mut buf = BytesMut::new();
        self.buffer_descriptions_ref(&mut buf)?;
        Ok(buf.freeze())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        use super::*;
        use vrpn_base::types::{RemoteId, SenderId};
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
