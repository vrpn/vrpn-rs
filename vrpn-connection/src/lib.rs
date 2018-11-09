// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate vrpn_base;
extern crate vrpn_buffer;

extern crate bytes;

#[macro_use]
extern crate downcast_rs;

#[macro_use]
extern crate quick_error;

pub mod endpoint;
pub mod error;
pub mod translation;
pub mod translationtable;
pub mod typedispatcher;

pub use crate::{
    endpoint::*,
    error::{append_error, Error, Result},
    translationtable::{MatchingTable, Table as TranslationTable, Tables as TranslationTables},
    typedispatcher::{Handler, RegisterMapping, SystemHandler, TypeDispatcher},
};

pub mod prelude {
    pub use crate::translationtable::MatchingTable;
}
