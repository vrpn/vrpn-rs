// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    size::ConstantBufferSize,
    unbuffer::{check_unbuffer_remaining, UnbufferFrom},
    BufferResult, BufferTo, UnbufferResult,
};
use bytes::{Buf, BufMut};

macro_rules! buffer_primitive {
    ($t:ty, $put:ident, $get:ident) => {
        impl ConstantBufferSize for $t {}

        impl BufferTo for $t {
            fn buffer_to<T: BufMut>(&self, buf: &mut T) -> BufferResult {
                buf.$put(*self);
                Ok(())
            }
        }

        impl UnbufferFrom for $t {
            fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
                check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
                Ok(buf.$get())
            }
        }
    };
}

buffer_primitive!(i8, put_i8, get_i8);
buffer_primitive!(i16, put_i16, get_i16);
buffer_primitive!(u16, put_u16, get_u16);
buffer_primitive!(i32, put_i32, get_i32);
buffer_primitive!(u32, put_u32, get_u32);
buffer_primitive!(i64, put_i64, get_i64);
buffer_primitive!(u64, put_u64, get_u64);
buffer_primitive!(f32, put_f32, get_f32);
buffer_primitive!(f64, put_f64, get_f64);

impl ConstantBufferSize for () {
    fn constant_buffer_size() -> usize {
        0
    }
}

impl BufferTo for () {
    fn buffer_to<T: BufMut>(&self, _buf: &mut T) -> BufferResult {
        Ok(())
    }
}

impl UnbufferFrom for () {
    fn unbuffer_from<T: Buf>(_buf: &mut T) -> UnbufferResult<Self> {
        Ok(())
    }
}
