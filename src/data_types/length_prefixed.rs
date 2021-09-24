// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes};
use std::mem::size_of;

use crate::buffer_unbuffer::{
    buffer::{self, BufferTo},
    size_requirement::*,
    unbuffer::{self, UnbufferFrom},
};

/// Does the "length prefix" value include a trailing null character (strlen() + 1)?
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LengthBehavior {
    /// Length is strlen + 1
    IncludeNull,
    /// Length is strlen
    ExcludeNull,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NullTermination {
    AddTrailingNull,
    NoNull,
}

/// Get the size required to buffer a string, preceded by its length and followed by a null byte.
pub fn buffer_size(s: &[u8], termination: NullTermination) -> usize {
    size_of::<u32>()
        + s.len()
        + match termination {
            NullTermination::NoNull => 0,
            NullTermination::AddTrailingNull => 1,
        }
}

/// Buffer a string, preceded by its length and followed by a null bytes.
pub fn buffer_string<T: BufMut>(
    s: &[u8],
    buf: &mut T,
    termination: NullTermination,
    null_in_len: LengthBehavior,
) -> buffer::BufferResult {
    let mut buf_size = buffer_size(s, termination);

    buffer::check_buffer_remaining(buf, buf_size)?;
    if termination == NullTermination::AddTrailingNull && null_in_len == LengthBehavior::ExcludeNull
    {
        // Decrement the length that we transmit if we're adding a null terminator but not including it in the length.
        buf_size -= 1;
    }
    let buf_size = buf_size as u32;
    buf_size.buffer_to(buf)?;

    buf.put(s);
    buf.put_u8(0);
    Ok(())
}

/// Unbuffer a string, preceded by its length and followed by a null bytes.
pub fn unbuffer_string<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Bytes> {
    let buf_size =
        u32::unbuffer_from(buf).map_err(ExpandSizeRequirement::expand_size_requirement)?;

    let buf_size = buf_size as usize;
    unbuffer::check_unbuffer_remaining(buf, buf_size)?;
    assert_ne!(
        buf_size, 0,
        "length-prefixed string size is expected to be greater than 0"
    );
    // Subtract null-terminator from length we want.
    let buf_size = buf_size - 1;

    let s = buf.copy_to_bytes(buf_size);
    // Grab null terminator
    unbuffer::consume_expected(buf, b"\0")?;
    Ok(s)
}
