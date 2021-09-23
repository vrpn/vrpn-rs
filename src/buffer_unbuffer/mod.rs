// Copyright 2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Routines and traits to buffer/unbuffer to/from byte buffers.

pub mod buffer;
pub mod constants;
mod error;
mod primitives;
mod size;
pub mod size_requirement;
pub mod unbuffer;

#[doc(inline)]
pub use crate::buffer_unbuffer::{
    error::BufferUnbufferError,
    primitives::*,
    size::{BufferSize, ConstantBufferSize, EmptyMessage, WrappedConstantSize},
};

pub use crate::buffer_unbuffer::{
    buffer::{check_buffer_remaining, Buffer, BufferResult, BytesMutExtras},
    size_requirement::SizeRequirement,
    unbuffer::{
        check_unbuffer_remaining, consume_expected, unbuffer_decimal_digits, Unbuffer,
        UnbufferResult, peek_u32
    },
};
