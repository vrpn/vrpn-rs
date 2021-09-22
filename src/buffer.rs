// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Extension traits related to buffering types.

use crate::{BufferSize, EmptyResult, Result, WrappedConstantSize};
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
    fn allocate_and_buffer<T: Buffer>(self, v: T) -> Result<Self>;
}

impl BytesMutExtras for BytesMut {
    fn allocate_and_buffer<T: Buffer>(mut self, v: T) -> Result<Self> {
        self.reserve(v.buffer_size());
        v.buffer_ref(&mut self)?;
        Ok(self)
    }
}

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait Buffer: BufferSize {
    /// Serialize to a buffer (taken as a mutable reference)
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult;

    /// Get the number of bytes required to serialize this to a buffer.
    fn required_buffer_size(&self) -> usize {
        self.buffer_size()
    }
}

impl<T: WrappedConstantSize> Buffer for T {
    fn buffer_ref<U: BufMut>(&self, buf: &mut U) -> EmptyResult {
        self.get().buffer_ref(buf)
    }
}
