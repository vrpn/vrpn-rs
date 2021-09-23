// Copyright 2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Data types

mod constants;
mod cookie;
mod descriptions;
mod length_prefixed;
mod message;
mod time;
mod types;

#[doc(inline)]
pub use crate::data_types::{
    cookie::{CookieData, Version},
    descriptions::{Description, UdpDescription},
    message::{
        GenericBody, GenericMessage, Message, MessageBody, MessageHeader, MessageTypeIdentifier,
        MessageTypeIdentifier::UserMessageName, SequencedGenericMessage, SequencedMessage,
        TypedMessageBody,
    },
    time::TimeVal,
    types::*,
};
