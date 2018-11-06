// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::traits::{
    buffer::{self, Buffer},
    unbuffer::{self, Source, UnbufferConstantSize},
    ConstantBufferSize,
};
use bytes::{Buf, BufMut, Bytes, IntoBuf};

macro_rules! buffer_primitive {
    ($t:ty, $put:ident, $get:ident) => {
        impl ConstantBufferSize for $t {}

        impl Buffer for $t {
            fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
                buf.$put(*self);
                Ok(())
            }
        }

        impl UnbufferConstantSize for $t {
            fn unbuffer_constant_size<T: Source>(buf: T) -> unbuffer::Result<Self> {
                Ok(buf.into_buf().$get())
            }
        }
    };
}

buffer_primitive!(i8, put_i8, get_i8);
buffer_primitive!(i16, put_i16_be, get_i16_be);
buffer_primitive!(u16, put_u16_be, get_u16_be);
buffer_primitive!(i32, put_i32_be, get_i32_be);
buffer_primitive!(u32, put_u32_be, get_u32_be);
buffer_primitive!(i64, put_i64_be, get_i64_be);
buffer_primitive!(u64, put_u64_be, get_u64_be);
buffer_primitive!(f32, put_f32_be, get_f32_be);
buffer_primitive!(f64, put_f64_be, get_f64_be);
