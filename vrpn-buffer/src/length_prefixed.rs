// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
// use nom_wrapper::call_nom_parser;
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;
use std::num::ParseIntError;
use std::result;
use traits::{
    buffer::{self, Buffer},
    unbuffer::{self, Output, OutputResultExtras, Unbuffer},
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
/*
/// Buffer a string, preceded by its length and followed by a null bytes.
pub fn unbuffer_string(buf: Bytes) -> unbuffer::Result<Bytes> {
    let buf_size : u32 = Unbuffer::unbuffer(buf)buffer_size(s);
    if buf.remaining_mut() < buf_size {
        return Err(buffer::Error::OutOfBuffer);
    }
    let len_size = size_of::<u32>();
    if buf.remaining() < size_of::<u32>() {
        return Err(unbuffer::Error::NeedMoreData(BytesRequired::AtLeast));
    }
    let buf_size: u32 = buf_size as u32;
    buf_size.buffer(buf).and_then(|| {
        buf.put(s);
        buf.put_u8(0);
        Ok(())
    })
}
*/
