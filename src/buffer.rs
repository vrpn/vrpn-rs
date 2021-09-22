// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Extension traits related to buffering types.

use crate::{BufferSize, BufferUnbufferError, BytesRequired, WrappedConstantSize};
use bytes::{BufMut, BytesMut};

/// Extension trait for BytesMut for easier interaction with stuff we can buffer.
pub trait BytesMutExtras
where
    Self: Sized,
{
    /// Allocate enough space in the buffer for the given value, then serialize the value to the buffer.
    ///
    /// # Errors
    /// If buffering fails.
    fn allocate_and_buffer<T: Buffer>(self, v: T)
        -> std::result::Result<Self, BufferUnbufferError>;
}

impl BytesMutExtras for BytesMut {
    fn allocate_and_buffer<T: Buffer>(
        mut self,
        v: T,
    ) -> std::result::Result<Self, BufferUnbufferError> {
        self.reserve(v.buffer_size());
        v.buffer_ref(&mut self)?;
        Ok(self)
    }
}

/// Shorthand name for what a buffering operation should return.
pub type BufferResult = std::result::Result<(), BufferUnbufferError>;

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait Buffer: BufferSize {
    /// Serialize to a buffer (taken as a mutable reference)
    ///
    /// Implementations must call `check_buffer_remaining(...)?;` first
    /// or otherwise avoid modifying the buffer if the whole message cannot fit!
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> BufferResult;

    /// Get the number of bytes required to serialize this to a buffer.
    fn required_buffer_size(&self) -> usize {
        self.buffer_size()
    }
}

impl<T: WrappedConstantSize> Buffer for T {
    fn buffer_ref<U: BufMut>(&self, buf: &mut U) -> BufferResult {
        self.get().buffer_ref(buf)
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
