// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Math types used across VRPN.

use crate::buffer_unbuffer::{buffer, unbuffer, ConstantBufferSize};
use bytes::{Buf, BufMut};

/// A 3D vector of 64-bit floats
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl ConstantBufferSize for Vec3 {
    fn constant_buffer_size() -> usize {
        std::mem::size_of::<f64>() * 3
    }
}

impl buffer::BufferTo for Vec3 {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.x.buffer_to(buf)?;
        self.y.buffer_to(buf)?;
        self.z.buffer_to(buf)?;
        Ok(())
    }
}

impl unbuffer::UnbufferFrom for Vec3 {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        unbuffer::check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let x = f64::unbuffer_from(buf)?;
        let y = f64::unbuffer_from(buf)?;
        let z = f64::unbuffer_from(buf)?;
        Ok(Vec3::new(x, y, z))
    }
}

/// A (typically unit) quaternion corresponding to a rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub s: f64,
    pub v: Vec3,
}

impl Quat {
    /// Create from scalar part and vector part.
    pub fn from_sv(s: f64, v: Vec3) -> Quat {
        Quat { s, v }
    }

    /// Create from all four coefficients: mind the order!
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Quat {
        Quat {
            s: w,
            v: Vec3::new(x, y, z),
        }
    }

    /// Return an identity rotation
    pub fn identity() -> Quat {
        Quat {
            s: 1.0,
            v: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl ConstantBufferSize for Quat {
    fn constant_buffer_size() -> usize {
        std::mem::size_of::<f64>() * 4
    }
}

impl buffer::BufferTo for Quat {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.v.buffer_to(buf)?;
        self.s.buffer_to(buf)?;
        Ok(())
    }
}

impl unbuffer::UnbufferFrom for Quat {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        unbuffer::check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let v = Vec3::unbuffer_from(buf)?;
        let w = f64::unbuffer_from(buf)?;
        Ok(Quat::from_sv(w, v))
    }
}
