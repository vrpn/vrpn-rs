// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::size_requirement::{
        ExpandSizeRequirement, MayContainSizeRequirement, SizeRequirement,
    },
    buffer_unbuffer::{BufferUnbufferError, MessageSizeInvalid},
    data_types::id_types::IdType,
};

use thiserror::Error;

/// Error type for the main VRPN crate
#[derive(Error, Debug)]
pub enum VrpnError {
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
    MessageSizeInvalid(MessageSizeInvalid),
    #[error("{0}")]
    VersionMismatch(crate::data_types::cookie::VersionMismatch),
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send>),
    #[error("{0}")]
    OtherMessage(String),
}

impl From<std::io::Error> for VrpnError {
    fn from(e: std::io::Error) -> Self {
        VrpnError::Other(Box::new(e))
    }
}

impl From<crate::data_types::cookie::VersionMismatch> for VrpnError {
    fn from(e: crate::data_types::cookie::VersionMismatch) -> Self {
        VrpnError::VersionMismatch(e)
    }
}
impl MayContainSizeRequirement for VrpnError {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            VrpnError::BufferUnbuffer(e) => e.try_get_size_requirement(),
            _ => None,
        }
    }
}

impl MayContainSizeRequirement for &VrpnError {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        match self {
            VrpnError::BufferUnbuffer(e) => e.try_get_size_requirement(),
            _ => None,
        }
    }
}

impl ExpandSizeRequirement for VrpnError {
    /// Maps `BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(n))` to
    /// `BufferUnbufferError::NeedMoreData(BytesRequired::AtLeast(n))`
    fn expand_size_requirement(self) -> Self {
        match self {
            VrpnError::BufferUnbuffer(e) => VrpnError::BufferUnbuffer(e.expand_size_requirement()),
            _ => self,
        }
    }
}

impl From<SizeRequirement> for VrpnError {
    fn from(val: SizeRequirement) -> Self {
        VrpnError::BufferUnbuffer(BufferUnbufferError::from(val))
    }
}

impl VrpnError {
    /// Maps `BufferUnbufferError::NeedMoreData(_)` to `BufferUnbufferError::HeaderSizeMismatch(_)`
    pub fn map_bytes_required_to_size_mismatch(self) -> VrpnError {
        match self {
            VrpnError::BufferUnbuffer(e) => {
                VrpnError::BufferUnbuffer(e.map_bytes_required_to_size_mismatch())
            }
            _ => self,
        }
    }
}

impl VrpnError {
    pub fn is_need_more_data(&self) -> bool {
        self.try_get_size_requirement().is_some()
    }
}

impl<T> From<std::sync::PoisonError<T>> for VrpnError {
    fn from(v: std::sync::PoisonError<T>) -> VrpnError {
        VrpnError::OtherMessage(v.to_string())
    }
}

impl From<MessageSizeInvalid> for VrpnError {
    fn from(_: MessageSizeInvalid) -> Self {
        todo!()
    }
}
// #[deprecated(note = "Use std::result::Result with explicit error type instead")]
pub type Result<T> = std::result::Result<T, VrpnError>;

#[deprecated(note = "You probably want crate::buffer_unbuffer::buffer::BufferResult")]
pub type EmptyResult = Result<()>;
