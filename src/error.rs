// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{size_requirement::SizeRequirement, IdType, Version};
use bytes::Bytes;
use std::{
    convert::TryFrom,
    fmt::{self, Display},
    net::AddrParseError,
    num::ParseIntError,
    ops::Add,
};
use thiserror::Error;

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
}

impl From<SizeRequirement> for BufferUnbufferError {
    fn from(val: SizeRequirement) -> Self {
        BufferUnbufferError::NeedMoreData(val)
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

impl TryFrom<BufferUnbufferError> for SizeRequirement {
    type Error = DoesNotContainBytesRequired;

    fn try_from(value: BufferUnbufferError) -> std::result::Result<Self, Self::Error> {
        if let BufferUnbufferError::NeedMoreData(required) = value {
            Ok(required)
        } else {
            Err(DoesNotContainBytesRequired(()))
        }
    }
}

impl TryFrom<&BufferUnbufferError> for SizeRequirement {
    type Error = DoesNotContainBytesRequired;

    fn try_from(value: &BufferUnbufferError) -> std::result::Result<Self, Self::Error> {
        if let BufferUnbufferError::NeedMoreData(required) = value {
            Ok(*required)
        } else {
            Err(DoesNotContainBytesRequired(()))
        }
    }
}

impl BufferUnbufferError {
    /// Maps `BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(n))` to
    /// `BufferUnbufferError::NeedMoreData(BytesRequired::AtLeast(n))`
    pub fn expand_bytes_required(self) -> BufferUnbufferError {
        if let BufferUnbufferError::NeedMoreData(required) = self {
            return BufferUnbufferError::NeedMoreData(required.expand());
        }
        self
    }

    /// Maps `BufferUnbufferError::NeedMoreData(_)` to `BufferUnbufferError::HeaderSizeMismatch(_)`
    pub fn map_bytes_required_to_size_mismatch(self) -> BufferUnbufferError {
        if let BufferUnbufferError::NeedMoreData(required) = self {
            return BufferUnbufferError::HeaderSizeMismatch(required.to_string());
        }
        self
    }
}

/// Error type for the main VRPN crate
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    BufferUnbuffer(#[from] BufferUnbufferError),
    #[error("invalid id {0}")]
    InvalidId(IdType),
    #[error("empty translation table entry")]
    EmptyEntry,
    #[error("too many handlers")]
    TooManyHandlers,
    #[error("too many mappings")]
    TooManyMappings,
    #[error("handler not found")]
    HandlerNotFound,
    #[error("handler returned an error")]
    GenericErrorReturn,
    #[error("a non-system message was forwarded to Endpoint::handle_message_as_system()")]
    NotSystemMessage,
    #[error("un-recognized system message id {0}")]
    UnrecognizedSystemMessage(IdType),
    #[error("version mismatch: expected something compatible with {expected}, got {actual}")]
    VersionMismatch { actual: Version, expected: Version },
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send>),
    #[error("{0}")]
    OtherMessage(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Other(Box::new(e))
    }
}

impl TryFrom<Error> for SizeRequirement {
    type Error = DoesNotContainBytesRequired;

    fn try_from(value: Error) -> std::result::Result<Self, Self::Error> {
        if let Error::BufferUnbuffer(buf_unbuf) = value {
            SizeRequirement::try_from(buf_unbuf)
        } else {
            Err(DoesNotContainBytesRequired(()))
        }
    }
}

impl TryFrom<&Error> for SizeRequirement {
    type Error = DoesNotContainBytesRequired;

    fn try_from(value: &Error) -> std::result::Result<Self, Self::Error> {
        if let Error::BufferUnbuffer(buf_unbuf) = value {
            SizeRequirement::try_from(buf_unbuf)
        } else {
            Err(DoesNotContainBytesRequired(()))
        }
    }
}

impl From<SizeRequirement> for Error {
    fn from(val: SizeRequirement) -> Self {
        Error::BufferUnbuffer(BufferUnbufferError::from(val))
    }
}

impl Error {
    /// Maps `Error::BufferUnbuffer(BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(n)))` to
    /// `Error::BufferUnbuffer((BufferUnbufferError::NeedMoreData(BytesRequired::AtLeast(n)))`
    pub fn expand_bytes_required(self) -> Error {
        if let Error::BufferUnbuffer(err) = self {
            return Error::BufferUnbuffer(err.expand_bytes_required());
        }
        self
    }
    /// Maps `BufferUnbufferError::NeedMoreData(_)` to `BufferUnbufferError::HeaderSizeMismatch(_)`
    pub fn map_bytes_required_to_size_mismatch(self) -> Error {
        if let Error::BufferUnbuffer(err) = self {
            return Error::BufferUnbuffer(err.map_bytes_required_to_size_mismatch());
        }
        self
    }
}

impl Error {
    // pub fn append(self, new_err: Error) -> Error {
    //     Error::ConsErrors(Box::new(new_err), Box::new(self))
    // }

    pub fn is_need_more_data(&self) -> bool {
        SizeRequirement::try_from(self).is_ok()
    }

    // pub fn contains_need_more_data(&self) -> bool {
    //     if self.is_need_more_data() { return true;}
    //     // let head : Option<Box<Error>> = None;
    //     // let tail: Option<Box<Error>> = Some(self);
    //     if let &Error::ConsErrors(head, tail) = self {
    //         return head.contains_need_more_data_internal() || tail.contains_need_more_data_internal();
    //     }
    //     return false;
    // }

    // fn contains_need_more_data_internal(self: &Box<Error>) -> bool {
    //     let mut tail: &Box<Error> = self;
    //     loop {
    //         if tail.is_need_more_data() { return true;}
    //         if let &Error::ConsErrors(head, new_tail) = tail {
    //         }
    //     }

    // }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(v: std::sync::PoisonError<T>) -> Error {
        Error::OtherMessage(v.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type EmptyResult = Result<()>;

// /// Combine a result with an error.
// ///
// /// If the result already is an error, the new error gets appended.
// pub fn append_error(old: Result<()>, new_err: Error) -> Result<()> {
//     match old {
//         Err(old_e) => Err(old_e.append(new_err)),
//         Ok(()) => Err(new_err),
//     }
// }
