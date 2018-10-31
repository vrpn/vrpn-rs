// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::Buffer;
use bytes::{BufMut, Bytes};
use size::ConstantBufferSize;
use unbuffer::{Result, UnbufferConstantSize};

pub trait WrappedConstantSize {
    type WrappedType: Buffer + UnbufferConstantSize + ConstantBufferSize;
    fn get(self) -> Self::WrappedType;
    fn create(v: Self::WrappedType) -> Self;
}

impl<T: WrappedConstantSize> Buffer for T {
    fn buffer<U: BufMut>(buf: &mut U, v: Self) {
        Buffer::buffer(buf, v.get())
    }
}

impl<T: WrappedConstantSize> ConstantBufferSize for T {
    fn constant_buffer_size() -> usize {
        T::WrappedType::constant_buffer_size()
    }
}

impl<T: WrappedConstantSize> UnbufferConstantSize for T {
    fn unbuffer_constant_size(buf: Bytes) -> Result<Self> {
        T::WrappedType::unbuffer_constant_size(buf).map(|v| T::create(v))
    }
}
