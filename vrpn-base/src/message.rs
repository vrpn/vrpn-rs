// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use time::TimeVal;
use types::{SenderId, SequenceNumber, TypeId};

/// A message with header information, ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Message<T> {
    pub time: TimeVal,
    pub message_type: TypeId,
    pub sender: SenderId,
    pub data: T,
    pub sequence_number: Option<SequenceNumber>,
}

/// A generic message, where the inner contents have already been serialized.
pub type GenericMessage = Message<Bytes>;

// Header is 5 i32s (padded to vrpn_ALIGN):
// - unpadded header size + unpadded body size
// - time stamp
// - sender
// - type
// body is padded out to vrpn_ALIGN

pub struct Description {
    name: Bytes,
}
