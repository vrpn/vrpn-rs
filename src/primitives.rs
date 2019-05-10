// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    unbuffer::Source, unbuffer::UnbufferConstantSize, Buffer, BytesRequired, ConstantBufferSize,
    EmptyResult, Error, Quat, Result, Sensor, Unbuffer, Vec3, WrappedConstantSize,
};
use bytes::{Buf, BufMut, Bytes};

macro_rules! buffer_primitive {
    ($t:ty, $put:ident, $get:ident) => {
        impl ConstantBufferSize for $t {}

        impl Buffer for $t {
            fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
                buf.$put(*self);
                Ok(())
            }
        }

        impl UnbufferConstantSize for $t {
            fn unbuffer_constant_size<T: Source>(buf: T) -> Result<Self> {
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

impl ConstantBufferSize for () {
    fn constant_buffer_size() -> usize {
        0
    }
}

impl Buffer for () {
    fn buffer_ref<T: BufMut>(&self, _buf: &mut T) -> EmptyResult {
        Ok(())
    }
}

impl UnbufferConstantSize for () {
    fn unbuffer_constant_size<T: Source>(_buf: T) -> Result<Self> {
        Ok(())
    }
}

impl ConstantBufferSize for Vec3 {
    fn constant_buffer_size() -> usize {
        std::mem::size_of::<f64>() * 3
    }
}
impl Buffer for Vec3 {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        if buf.remaining_mut() < Self::constant_buffer_size() {
            Err(Error::OutOfBuffer)?;
        }
        self.x.buffer_ref(buf)?;
        self.y.buffer_ref(buf)?;
        self.z.buffer_ref(buf)?;
        Ok(())
    }
}

impl Unbuffer for Vec3 {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
        if buf.len() < Self::constant_buffer_size() {
            Err(Error::NeedMoreData(BytesRequired::Exactly(
                Self::constant_buffer_size() - buf.len(),
            )))?;
        }
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
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        if buf.remaining_mut() < Self::constant_buffer_size() {
            Err(Error::OutOfBuffer)?;
        }
        self.v.buffer_ref(buf)?;
        self.s.buffer_ref(buf)?;
        Ok(())
    }
}

impl Unbuffer for Quat {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
        if buf.len() < Self::constant_buffer_size() {
            Err(Error::NeedMoreData(BytesRequired::Exactly(
                Self::constant_buffer_size() - buf.len(),
            )))?;
        }
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
