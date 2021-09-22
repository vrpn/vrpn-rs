// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::buffer::BufferResult;
use crate::prelude::*;
use crate::{
    unbuffer::consume_expected, Buffer, BufferUnbufferError, BytesRequired, EmptyResult, Error,
    Result, Unbuffer,
};
use bytes::{Buf, BufMut, Bytes};
use std::mem::size_of;

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
) -> BufferResult {
    let mut buf_size = buffer_size(s, termination);
    if buf.remaining_mut() < buf_size {
        return Err(BufferUnbufferError::OutOfBuffer);
    }
    if termination == NullTermination::AddTrailingNull && null_in_len == LengthBehavior::ExcludeNull
    {
        // Decrement the length that we transmit if we're adding a null terminator but not including it in the length.
        buf_size -= 1;
    }
    let buf_size = buf_size as u32;
    buf_size.buffer_ref(buf)?;

    buf.put(s);
    buf.put_u8(0);
    Ok(())
}

/// Unbuffer a string, preceded by its length and followed by a null bytes.
pub fn unbuffer_string<T: Buf>(buf: &mut T) -> std::result::Result<Bytes, BufferUnbufferError> {
    let buf_size = u32::unbuffer_ref(buf).map_exactly_err_to_at_least()?;

    let buf_size = buf_size as usize;
    if buf.remaining() < buf_size {
        return Err(BufferUnbufferError::NeedMoreData(BytesRequired::Exactly(
            buf_size - buf.remaining(),
        )));
    }
    assert_ne!(
        buf_size, 0,
        "length-prefixed string size is expected to be greater than 0"
    );
    // Subtract null-terminator from length we want.
    let buf_size = buf_size - 1;

    let s = buf.copy_to_bytes(buf_size);
    // Grab null terminator
    consume_expected(buf, b"\0")?;
    Ok(s)
}
