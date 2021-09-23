// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::size_requirement::{
        ExpandSizeRequirement, MayContainSizeRequirement, SizeRequirement,
    },
    buffer_unbuffer::BufferUnbufferError,
    data_types::id_types::IdType,
};

use thiserror::Error;

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
    #[error("{0}")]
    VersionMismatch(crate::data_types::cookie::VersionMismatch),
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

impl From<crate::data_types::cookie::VersionMismatch> for Error {
    fn from(e: crate::data_types::cookie::VersionMismatch) -> Self {
        Error::VersionMismatch(e)
    }
}
impl MayContainSizeRequirement for Error {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            Error::BufferUnbuffer(e) => e.try_get_size_requirement(),
            _ => None,
        }
    }
}

impl MayContainSizeRequirement for &Error {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            Error::BufferUnbuffer(e) => e.try_get_size_requirement(),
            _ => None,
        }
    }
}

impl ExpandSizeRequirement for Error {
    /// Maps `BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(n))` to
    /// `BufferUnbufferError::NeedMoreData(BytesRequired::AtLeast(n))`
    fn expand_size_requirement(self) -> Self {
        match self {
            Error::BufferUnbuffer(e) => Error::BufferUnbuffer(e.expand_size_requirement()),
            _ => self,
        }
    }
}

impl From<SizeRequirement> for Error {
    fn from(val: SizeRequirement) -> Self {
        Error::BufferUnbuffer(BufferUnbufferError::from(val))
    }
}

impl Error {
    /// Maps `BufferUnbufferError::NeedMoreData(_)` to `BufferUnbufferError::HeaderSizeMismatch(_)`
    pub fn map_bytes_required_to_size_mismatch(self) -> Error {
        match self {
            Error::BufferUnbuffer(e) => {
                Error::BufferUnbuffer(e.map_bytes_required_to_size_mismatch())
            }
            _ => self,
        }
    }
}

impl Error {
    pub fn is_need_more_data(&self) -> bool {
        self.try_get_size_requirement().is_some()
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(v: std::sync::PoisonError<T>) -> Error {
        Error::OtherMessage(v.to_string())
    }
}

#[deprecated(note = "Use std::result::Result with explicit error type instead")]
pub type Result<T> = std::result::Result<T, Error>;

#[deprecated(note = "You probably want crate::buffer_unbuffer::buffer::BufferResult")]
pub type EmptyResult = Result<()>;
