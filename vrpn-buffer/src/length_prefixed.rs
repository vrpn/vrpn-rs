// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;
use std::num::ParseIntError;
use std::result;
use traits::{
    buffer::{self, Buffer},
    unbuffer::{self, check_expected, Output, OutputResultExtras, Unbuffer},
    BytesRequired, ConstantBufferSize,
};

/// Get the size required to buffer a string, preceded by its length and followed by a null bytes.
pub fn buffer_size(s: &[u8]) -> usize {
    size_of::<u32>() + s.len() + 1
}

/// Buffer a string, preceded by its length and followed by a null bytes.
pub fn buffer_string<T: BufMut>(s: &[u8], buf: &mut T) -> buffer::Result {
    let buf_size = buffer_size(s);
    if buf.remaining_mut() < buf_size {
        return Err(buffer::Error::OutOfBuffer);
    }
    let buf_size = buf_size as u32;
    buf_size.buffer(buf).and_then(|()| {
        buf.put(s);
        buf.put_u8(0);
        Ok(())
    })
}

/// Unbuffer a string, preceded by its length and followed by a null bytes.
pub fn unbuffer_string(buf: &mut Bytes) -> unbuffer::Result<Bytes> {
    let buf_size: u32 = Unbuffer::unbuffer(buf)
        .map_exactly_err_to_at_least()?
        .data();

    let buf_size = buf_size as usize;
    if buf.len() < buf_size {
        return Err(unbuffer::Error::NeedMoreData(BytesRequired::Exactly(
            buf_size - buf.len(),
        )));
    }
    assert_ne!(
        buf_size, 0,
        "length-prefixed string size is expected to be greater than 0"
    );
    // Subtract null-terminator from length we want.
    let buf_size = buf_size - 1;

    let s = buf.split_to(buf_size);
    // Grab null terminator
    check_expected(buf, b"\0")?;
    Ok(s)
}
