// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::BufMut;
use size::ConstantBufferSize;

/// Trait for computing the buffer size needed for types
/// that can be "buffered" (serialized to a byte buffer)
pub trait BufferSize {
    /// Indicates the number of bytes required in the buffer to store this.
    fn buffer_size(&self) -> usize;
}

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait Buffer: BufferSize {
    /// Serialize to a buffer.
    fn buffer<T: BufMut>(buf: &mut T, v: Self);
}

impl<T: ConstantBufferSize> BufferSize for T {
    fn buffer_size(&self) -> usize {
        T::constant_buffer_size()
    }
}
