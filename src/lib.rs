// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;
extern crate cgmath;
extern crate chrono;
extern crate tk_listen;
extern crate url;

#[cfg(test)]
#[macro_use]
extern crate hex_literal;
#[cfg(test)]
#[macro_use]
extern crate proptest;

#[macro_use]
extern crate bitmask;

#[macro_use]
extern crate downcast_rs;

#[macro_use]
extern crate futures;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate tokio;

pub mod async_io;
pub mod buffer;
pub mod connection;
pub mod constants;
pub mod cookie;
pub mod descriptions;
pub mod endpoint;
pub mod error;
pub mod handler;
pub mod length_prefixed;
pub mod log;
pub mod message;
mod parse_name;
pub mod ping;
pub mod prelude;
pub mod primitives;
pub mod size;
pub mod time;
pub mod tracker;
pub mod translation_table;
pub mod type_dispatcher;
pub mod types;
pub mod unbuffer;
pub mod codec;

pub use crate::{
    buffer::{BufMutExtras, Buffer, BytesMutExtras},
    connection::{Connection, ConnectionStatus},
    cookie::{CookieData, Version},
    descriptions::{Description, UdpDescription},
    endpoint::*,
    error::*,
    handler::{Handler, TypedBodylessHandler, TypedHandler},
    log::{LogFileNames, LogFlags, LogMode},
    message::{
        GenericBody, GenericMessage, Message, MessageBody, MessageHeader, MessageTypeIdentifier,
        MessageTypeIdentifier::UserMessageName, SequencedGenericMessage, SequencedMessage,
        TypedMessageBody,
    },
    parse_name::{Scheme, ServerInfo},
    primitives::*,
    size::{BufferSize, ConstantBufferSize, EmptyMessage, WrappedConstantSize},
    time::TimeVal,
    type_dispatcher::{RegisterMapping, TypeDispatcher},
    types::*,
    unbuffer::{BytesExtras, OutputResultExtras, Unbuffer, UnbufferOutput},
};

pub(crate) use crate::{
    translation_table::{MatchingTable, Tables as TranslationTables},
    types::{determine_id_range, RangedId},
};
