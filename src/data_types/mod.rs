// Copyright 2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Data types

pub mod constants;
pub mod cookie;
pub(crate) mod descriptions;
pub mod id_types;
pub(crate) mod length_prefixed;
pub(crate) mod log;
mod math;
pub(crate) mod message;
pub mod name_types;
mod time;

#[cfg(cgmath)]
pub mod math_cgmath;

#[doc(inline)]
pub use crate::data_types::{
    cookie::{CookieData, Version},
    descriptions::{Description, UdpDescription},
    math::{Quat, Vec3},
    time::TimeVal,
};
pub use crate::data_types::{
    id_types::MessageTypeId,
    message::{
        GenericBody, GenericMessage, Message, MessageHeader, MessageSize, SequencedGenericMessage,
        TypedMessage, TypedMessageBody,
    },
    name_types::{
        IdWithNameAndDescription, MessageTypeIdentifier, MessageTypeName, SenderName,
        StaticMessageTypeName, StaticSenderName,
    },
};

pub(crate) use crate::data_types::log::{LogFileNames, LogMode};

bitflags! {
    /// Class of service flags matching those in the original vrpn
    pub struct ClassOfService : u32 {
        /// Results in TCP transport if available
        const RELIABLE = (1 << 0);
        const FIXED_LATENCY = (1 << 1);
        /// Results in UDP transport if available
        const LOW_LATENCY = (1 << 2);
        const FIXED_THROUGHPUT = (1 << 3);
        const HIGH_THROUGHPUT = (1 << 4);
    }
}
