// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Extension traits related to buffering types.

use bytes::{BufMut, BytesMut};

use super::{BufferSize, BufferUnbufferError, WrappedConstantSize};

/// Extension trait for BytesMut for easier interaction with stuff we can buffer.
pub trait BytesMutExtras
where
    Self: Sized,
{
    /// Allocate enough space in the buffer for the given value, then serialize the value to the buffer.
    ///
    /// # Errors
    /// If buffering fails.
    fn allocate_and_buffer<T: BufferTo>(v: T) -> std::result::Result<Self, BufferUnbufferError>;
}

impl BytesMutExtras for BytesMut {
    fn allocate_and_buffer<T: BufferTo>(v: T) -> std::result::Result<Self, BufferUnbufferError> {
        let mut buf = Self::with_capacity(v.buffer_size());
        v.buffer_to(&mut buf)?;
        Ok(buf)
    }
}

/// Shorthand name for what a buffering operation should return.
pub type BufferResult = std::result::Result<(), BufferUnbufferError>;

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait BufferTo: BufferSize {
    /// Serialize to a buffer (taken as a mutable reference)
    ///
    /// Buffer guaranteed big enough.
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> BufferResult;

    /// Get the number of bytes required to serialize this to a buffer.
    fn required_buffer_size(&self) -> usize {
        self.buffer_size()
    }
}

impl<T: WrappedConstantSize> BufferTo for T {
    fn buffer_to<U: BufMut>(&self, buf: &mut U) -> BufferResult {
        self.get().buffer_to(buf)
    }
}

/// Check whether a buffer has enough bytes remaining to unbuffer a given length
pub fn check_buffer_remaining<T: BufMut>(buf: &mut T, required_len: usize) -> BufferResult {
    let bytes_len = buf.remaining_mut();
    if bytes_len < required_len {
        Err(BufferUnbufferError::OutOfBuffer)
    } else {
        Ok(())
    }
}
