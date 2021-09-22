// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer::{check_buffer_remaining, BufferResult},
    unbuffer::UnbufferConstantSize,
    unbuffer::{check_unbuffer_remaining, UnbufferResult},
    Buffer, BufferUnbufferError, BytesRequired, ConstantBufferSize, Quat, Sensor, Unbuffer, Vec3,
    WrappedConstantSize,
};
use bytes::{Buf, BufMut};

macro_rules! buffer_primitive {
    ($t:ty, $put:ident, $get:ident) => {
        impl ConstantBufferSize for $t {}

        impl Buffer for $t {
            fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> BufferResult {
                check_buffer_remaining(buf, Self::constant_buffer_size())?;
                buf.$put(*self);
                Ok(())
            }
        }

        impl UnbufferConstantSize for $t {
            fn unbuffer_constant_size<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
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

impl Buffer for () {
    fn buffer_ref<T: BufMut>(&self, _buf: &mut T) -> BufferResult {
        Ok(())
    }
}

impl UnbufferConstantSize for () {
    fn unbuffer_constant_size<T: Buf>(_buf: &mut T) -> UnbufferResult<Self> {
        Ok(())
    }
}

impl ConstantBufferSize for Vec3 {
    fn constant_buffer_size() -> usize {
        std::mem::size_of::<f64>() * 3
    }
}

impl Buffer for Vec3 {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.x.buffer_ref(buf)?;
        self.y.buffer_ref(buf)?;
        self.z.buffer_ref(buf)?;
        Ok(())
    }
}

impl Unbuffer for Vec3 {
    fn unbuffer_ref<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let x = f64::unbuffer_ref(buf)?;
        let y = f64::unbuffer_ref(buf)?;
        let z = f64::unbuffer_ref(buf)?;
        Ok(Vec3::new(x, y, z))
    }
}

impl ConstantBufferSize for Quat {
    fn constant_buffer_size() -> usize {
        std::mem::size_of::<f64>() * 4
    }
}

impl Buffer for Quat {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.v.buffer_ref(buf)?;
        self.s.buffer_ref(buf)?;
        Ok(())
    }
}

impl Unbuffer for Quat {
    fn unbuffer_ref<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let v = Vec3::unbuffer_ref(buf)?;
        let w = f64::unbuffer_ref(buf)?;
        Ok(Quat::from_sv(w, v))
    }
}

impl WrappedConstantSize for Sensor {
    type WrappedType = i32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Sensor(v)
    }
}
