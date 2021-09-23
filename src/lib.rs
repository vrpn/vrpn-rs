// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;
extern crate cgmath;
extern crate chrono;
extern crate url;

// #[cfg(feature = "async-tokio")]
// extern crate tk_listen;

#[cfg(test)]
#[macro_use]
extern crate hex_literal;

#[cfg(test)]
#[macro_use]
extern crate proptest;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate downcast_rs;

extern crate futures;

#[cfg(feature = "async-tokio")]
extern crate tokio;

#[cfg(feature = "async-tokio")]
pub mod async_io;

pub mod buffer_unbuffer;
pub mod data_types;

pub mod connection;
pub mod constants;
pub mod descriptions;
mod codec;
pub mod endpoint;
pub mod error;
pub mod handler;
pub mod log;
mod parse_name;
pub mod ping;
pub mod prelude;
pub mod sync_io;
pub mod tracker;
pub mod translation_table;
pub mod type_dispatcher;

pub use crate::{
    connection::{Connection, ConnectionStatus},
    descriptions::{Description, UdpDescription},
    endpoint::*,
    error::{EmptyResult, Error, Result},
    handler::{Handler, TypedBodylessHandler, TypedHandler},
    log::{LogFileNames, LogMode},
    parse_name::{Scheme, ServerInfo},
    type_dispatcher::{RegisterMapping, TypeDispatcher},
};

pub(crate) use crate::translation_table::{MatchingTable, Tables as TranslationTables};
