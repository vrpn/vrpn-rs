// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes};
use itertools::join;
use itertools::Itertools;
use libc::timeval;
use std::mem::size_of;
use std::num::ParseIntError;

quick_error!{
#[derive(Debug)]
pub enum UnbufferError {
    BufferTooShort(actual: usize, expected: usize) {
        display("ran out of buffered bytes: expected at least {} bytes, but only had {}", expected, actual)
    }
    InvalidDecimalDigit(chars: Vec<char>) {
        display(self_) -> ("got the following non-decimal-digit(s) {}", join(chars.iter().map(|x : &char| x.to_string()), ","))
    }
    UnexpectedAsciiData(actual: Bytes, expected: Bytes) {
        display("unexpected data: expected '{:?}', got '{:?}'", &expected[..], &actual[..])
    }
    ParseInt(err: ParseIntError) {
        cause(err)
        description(err.description())
        display("{}", err)
        from()
    }
}
}

pub type BufferResult<T> = std::result::Result<T, UnbufferError>;

/// Optional trait for things that always take the same amount of space in a buffer.
pub trait ConstantBufferSize {
    /// Get the amount of space needed in a buffer.
    fn buffer_size() -> usize;
}

/// Trait to get the size needed for types that can be "buffered" (serialized to a byte buffer)
pub trait BufferSize {
    /// Indicates the number of bytes required in the buffer to store this.
    fn size_needed(&self) -> usize;
}

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait Buffer {
    /// Serialize to a buffer.
    fn buffer<T: BufMut>(buf: &mut T, v: Self);
}

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait Unbuffer: Sized {
    fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<Self>;
}

impl<T: ConstantBufferSize> BufferSize for T {
    fn size_needed(&self) -> usize {
        T::buffer_size()
    }
}

/// Error if buf does not contain at least n bytes.
pub fn check_remaining<T: Buf>(buf: &T, n: usize) -> BufferResult<()> {
    if buf.remaining() < n {
        Err(UnbufferError::BufferTooShort(buf.remaining(), n))
    } else {
        Ok(())
    }
}

macro_rules! buffer_primitive {
    ($t:ty, $put:ident, $get:ident) => {
        impl ConstantBufferSize for $t {
            fn buffer_size() -> usize {
                size_of::<Self>()
            }
        }

        impl Buffer for $t {
            fn buffer<T: BufMut>(buf: &mut T, v: $t) {
                buf.$put(v);
            }
        }

        impl Unbuffer for $t {
            fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<$t> {
                check_remaining(buf, size_of::<Self>())?;
                Ok(buf.$get())
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

impl Buffer for timeval {
    fn buffer<T: BufMut>(buf: &mut T, v: timeval) {
        buf.put_i32_be(v.tv_sec as i32);
        buf.put_i32_be(v.tv_usec as i32);
    }
}

impl Unbuffer for timeval {
    fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<timeval> {
        check_remaining(buf, 2 * size_of::<u32>())?;
        let sec = buf.get_i32_be();
        let usec = buf.get_i32_be();
        Ok(timeval {
            tv_sec: sec as i64,
            tv_usec: usec as i64,
        })
    }
}

/// Consume all remaining bytes in a buffer, parsing them as an ASCII decimal.
/*
pub(crate) fn decode_decimal<T: std::ops::MulAssign + std::ops::AddAssign, U: Buf>(
    buf: U,
) -> BufferResult<T> {
    let mut val = T::from(0);
    while buf.has_remaining() {
        let c = buf.get_u8();
        buf.advance(1);
        if c < ('0' as u8) || c > ('9' as u8) {
            return Err(UnbufferError::InvalidDecimalDigit(c));
        }
        val *= 10;
        val += c - ('0' as u8);
    }
    Ok(val)
}
*/

/// Consume as many bytes as the expected byte string contains, and error if they don't match.
pub fn check_expected<T: Buf>(buf: &mut T, expected: &'static [u8]) -> BufferResult<()> {
    let len = expected.len();
    check_remaining(buf, len)?;
    let take = buf.take(len);
    if take.bytes() == expected {
        return Ok(());
    }
    Err(UnbufferError::UnexpectedAsciiData(
        take.collect(),
        Bytes::from_static(expected),
    ))
}
