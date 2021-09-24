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
pub mod vrpn_tokio;

pub mod buffer_unbuffer;
pub mod data_types;

mod codec;
pub mod connection;
pub mod constants;
pub mod endpoint;
pub mod error;
pub mod handler;
mod parse_name;
pub mod ping;
#[deprecated]
pub mod prelude;
pub mod sync_io;
pub mod tracker;
pub mod translation_table;
pub mod type_dispatcher;

pub use crate::{
    connection::{Connection, ConnectionStatus},
    endpoint::*,
    error::{Result, VrpnError},
    handler::{Handler, TypedBodylessHandler, TypedHandler},
    parse_name::{Scheme, ServerInfo},
    type_dispatcher::{RegisterMapping, TypeDispatcher},
};

pub(crate) use crate::translation_table::{MatchingTable, Tables as TranslationTables};
