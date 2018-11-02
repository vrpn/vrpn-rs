// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;

extern crate itertools;

extern crate libc;

#[macro_use]
extern crate quick_error;

extern crate vrpn_base;

pub mod cookie;
pub mod length_prefixed;
pub mod message;
pub mod prelude;
pub mod primitives;
pub mod time;
pub mod traits;

pub use self::{
    primitives::*,
    traits::{
        buffer::{self, Buffer},
        unbuffer::{self, Output, Unbuffer, UnbufferConstantSize},
        BufferSize, ConstantBufferSize,
    },
};
