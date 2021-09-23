// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Traits describing the size of things we can read from or write to a buffer.

use super::{unbuffer::UnbufferConstantSize, Buffer};

/// Optional trait for things that always take the same amount of space in a buffer.
///
/// Implementing this trait gets you implementations of a bunch of buffer/unbuffer-related traits for free.
pub trait ConstantBufferSize {
    /// Get the amount of space needed in a buffer.
    /// TODO: some way to make this a constant?
    fn constant_buffer_size() -> usize
    where
        Self: Sized,
    {
        std::mem::size_of::<Self>()
    }
}

/// Trait for computing the buffer size needed for types
/// that can be "buffered" (serialized to a byte buffer),
pub trait BufferSize {
    /// Indicates the number of bytes required in the buffer to store this.
    fn buffer_size(&self) -> usize;
}

impl<T: ConstantBufferSize> BufferSize for T {
    fn buffer_size(&self) -> usize {
        T::constant_buffer_size()
    }
}

/// Trait for types that are a wrapper around some basic constant sized thing, like an ID.
pub trait WrappedConstantSize {
    type WrappedType: Buffer + UnbufferConstantSize + ConstantBufferSize;

    /// Get the wrapped value
    fn get(&self) -> Self::WrappedType;
    /// Create from a wrapped value
    fn new(v: Self::WrappedType) -> Self;
}

/// All WrappedConstantSize are also constant size (trivially)
impl<T: WrappedConstantSize> ConstantBufferSize for T {
    fn constant_buffer_size() -> usize {
        T::WrappedType::constant_buffer_size()
    }
}

/// Trait implemented by empty messages (no body)
/// so that they can easily get their trivial/null serialization support.
pub trait EmptyMessage: Default + std::fmt::Debug {}

/// Empty messages are effectively a wrapped constant size type.
impl<T: EmptyMessage> WrappedConstantSize for T {
    type WrappedType = ();
    fn get(&self) -> Self::WrappedType {}
    fn new(_v: Self::WrappedType) -> Self {
        Default::default()
    }
}
