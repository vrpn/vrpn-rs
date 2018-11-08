// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;
extern crate vrpn_base;
extern crate vrpn_buffer;

#[macro_use]
extern crate quick_error;

pub mod translationtable;
pub mod typedispatcher;

pub use crate::{
    translationtable::{Result as TranslationTableResult, TranslationTable, TranslationTableError},
    typedispatcher::{HandlerResult, MappingResult, RegisterMapping, TypeDispatcher},
};
