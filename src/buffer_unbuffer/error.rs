// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use std::{net::AddrParseError, num::ParseIntError};
use thiserror::Error;

use super::{
    size_requirement::{ExpandSizeRequirement, MayContainSizeRequirement},
    SizeRequirement,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct MessageSizeInvalid(pub u32);

impl std::fmt::Display for MessageSizeInvalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Message size field {} is smaller than minimum", self.0)
    }
}

impl std::error::Error for MessageSizeInvalid {}

/// Error type returned by buffering/unbuffering.
#[derive(Error, Debug)]
pub enum BufferUnbufferError {
    #[error("unbuffering ran out of buffered bytes: need {0} additional bytes")]
    NeedMoreData(SizeRequirement),
    #[error("unexpected data: expected '{expected:?}', got '{actual:?}'")]
    UnexpectedAsciiData { actual: Bytes, expected: Bytes },
    #[error("buffering ran out of buffer space")]
    OutOfBuffer,
    #[error("according to a length field we have complete data, but we need at least {0} additional bytes")]
    HeaderSizeMismatch(String),
    #[error("Error parsing {parsing_kind}: {s}")]
    ParseError { parsing_kind: String, s: String },
    #[error("{}", .0)]
    MessageSizeInvalid(MessageSizeInvalid),
}

impl From<SizeRequirement> for BufferUnbufferError {
    fn from(val: SizeRequirement) -> Self {
        BufferUnbufferError::NeedMoreData(val)
    }
}

impl From<MessageSizeInvalid> for BufferUnbufferError {
    fn from(val: MessageSizeInvalid) -> Self {
        BufferUnbufferError::MessageSizeInvalid(val)
    }
}

impl From<ParseIntError> for BufferUnbufferError {
    fn from(e: ParseIntError) -> Self {
        BufferUnbufferError::ParseError {
            parsing_kind: "integer".to_string(),
            s: e.to_string(),
        }
    }
}

impl From<AddrParseError> for BufferUnbufferError {
    fn from(e: AddrParseError) -> Self {
        BufferUnbufferError::ParseError {
            parsing_kind: "IP address".to_string(),
            s: e.to_string(),
        }
    }
}

impl MayContainSizeRequirement for BufferUnbufferError {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            BufferUnbufferError::NeedMoreData(required) => Some(required),
            _ => None,
        }
    }
}

impl MayContainSizeRequirement for &BufferUnbufferError {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            BufferUnbufferError::NeedMoreData(required) => Some(*required),
            _ => None,
        }
    }
}

impl ExpandSizeRequirement for BufferUnbufferError {
    /// Maps `BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(n))` to
    /// `BufferUnbufferError::NeedMoreData(BytesRequired::AtLeast(n))`
    fn expand_size_requirement(self) -> Self {
        use BufferUnbufferError::*;
        match self {
            NeedMoreData(required) => NeedMoreData(required.expand()),
            _ => self,
        }
    }
}

impl BufferUnbufferError {
    /// Maps `BufferUnbufferError::NeedMoreData(_)` to `BufferUnbufferError::HeaderSizeMismatch(_)`
    pub fn map_bytes_required_to_size_mismatch(self) -> BufferUnbufferError {
        use BufferUnbufferError::*;
        match self {
            NeedMoreData(required) => HeaderSizeMismatch(required.to_string()),
            _ => self,
        }
    }
}
