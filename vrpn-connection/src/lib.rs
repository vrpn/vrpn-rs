// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;
extern crate vrpn_base;
extern crate vrpn_buffer;

#[macro_use]
extern crate quick_error;

pub mod connection;
pub mod endpoint;
pub mod translationtable;
pub mod typedispatcher;

pub use crate::{
    connection::Connection,
    translationtable::{Result as TranslationTableResult, TranslationTable, TranslationTableError},
    typedispatcher::{HandlerResult, MappingResult, TypeDispatcher},
};
