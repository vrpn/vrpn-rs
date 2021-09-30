// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate pin_project_lite;

pub mod bytes_mut_reader;
pub mod connect;
pub mod cookie;
pub mod endpoint_ip;
mod endpoints;
pub mod message_stream;
mod unbounded_message_sender;

pub use message_stream::{AsyncReadMessagesExt, MessageStream};
pub(crate) use unbounded_message_sender::UnboundedMessageSender;
