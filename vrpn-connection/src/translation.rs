// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{prelude::*, translationtable::Entry, typedispatcher, Error, Result};
use vrpn_base::{
    message::{Description, SequencedMessage},
    types::{BaseTypeSafeId, IdType, LocalId, RemoteId, SenderId, TypeId, TypeSafeId},
};

/// Convert a remote ID to a local ID, if found.
pub fn map_to_local_id<T, U>(translation: &U, id: RemoteId<T>) -> Result<Option<LocalId<T>>>
where
    T: BaseTypeSafeId,
    U: MatchingTable<T>,
{
    translation.table().map_to_local_id(id)
}

/// Record a remote and local ID with the corresponding name.
pub fn add_remote_entry<T, U>(
    translation: &mut U,
    name: Bytes,
    remote_id: RemoteId<T>,
    local_id: LocalId<T>,
) -> Result<RemoteId<T>>
where
    T: BaseTypeSafeId,
    U: MatchingTable<T>,
{
    translation
        .table_mut()
        .add_remote_entry(name, remote_id, local_id)
}

/// Gets a shared borrow of an entry, given its local ID.
pub fn find_by_local_id<T, U>(translation: &mut U, local_id: LocalId<T>) -> Option<&Entry<T>>
where
    T: BaseTypeSafeId,
    U: MatchingTable<T>,
{
    translation
        .table()
        .find_by_predicate(|entry| entry.local_id() == local_id)
}
