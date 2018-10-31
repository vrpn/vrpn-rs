// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes, IntoBuf};
use itertools::join;
use itertools::Itertools;
use libc::timeval;
use std::mem::size_of;
use std::num::ParseIntError;
use std::ops::Add;

quick_error!{
#[derive(Debug)]
pub enum UnbufferError {
    BufferTooShort(actual: usize, expected: usize) {
        display("ran out of buffered bytes: expected at least {} bytes, but only had {}", expected, actual)
    }
    InvalidDecimalDigit(chars: Vec<char>) {
        display(self_) -> ("got the following non-decimal-digit(s) {}", join(chars.iter().map(|x : &char| x.to_string()), ","))
    }
    DecimalOverflow {
        description("overflow when parsing decimal digits")
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
///
/// Implementing this trait gets you implementations of a bunch of buffer/unbuffer-related traits for free.
pub trait ConstantBufferSize {
    /// Get the amount of space needed in a buffer.
    fn buffer_size() -> usize;
}

/// Trait for computing the buffer size needed for types
/// that can be "buffered" (serialized to a byte buffer)
pub trait BufferSize {
    /// Indicates the number of bytes required in the buffer to store this.
    fn size_needed(&self) -> usize;
}

/// Trait for types that can be "buffered" (serialized to a byte buffer)
pub trait Buffer: BufferSize {
    /// Serialize to a buffer.
    fn buffer<T: BufMut>(buf: &mut T, v: Self);
}

pub enum CapacityCheckOutcome {
    Ok(Bytes),
    NotEnoughData,
    Err(UnbufferError),
}

impl CapacityCheckOutcome {
    pub fn and_then_check<T: UnbufferCheckCapacity>(self) -> CapacityCheckOutcome {
        use self::CapacityCheckOutcome::*;
        match self {
            Ok(buf) => T::check_capacity(buf),
            NotEnoughData => NotEnoughData,
            Err(e) => Err(e),
        }
    }
}

pub enum BytesRequired {
    Constant(usize),
    Unknown,
}

impl BytesRequired {
    pub fn satisfied_by(&self, buf_size: usize) -> Option<bool> {
        match *self {
            BytesRequired::Constant(c) => Some(c <= buf_size),
            BytesRequired::Unknown => None,
        }
    }
}

impl Add for BytesRequired {
    type Output = BytesRequired;
    fn add(self, other: BytesRequired) -> Self::Output {
        use self::BytesRequired::*;
        match (self, other) {
            (Constant(a), Constant(b)) => Constant(a + b),
            _ => Unknown,
        }
    }
}

/// Trait for checking if a buffer contains a value of a type that can be "unbuffered" (parsed from a byte buffer)
pub trait UnbufferCheckCapacity: Sized {
    /// Checks if there is a full value stored here.
    ///
    /// Return CapacityCheckOutcome::Ok(buf) (where buf has been advanced) if there is,
    /// Ok(None) if there isn't, and Err() if there was enough bytes but
    /// it's invalid for some other reason.
    ///
    /// Not for direct usage.
    fn check_capacity(buf: Bytes) -> CapacityCheckOutcome;

    /// Returns the number of bytes required for this type in general.
    ///
    /// If size is variable, the default implementation will return BytesRequired::Unknown.
    fn bytes_required() -> BytesRequired {
        BytesRequired::Unknown
    }
}

pub struct BytesConsumed(pub usize);

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait Unbuffer: Sized + UnbufferCheckCapacity {
    /// Unbuffers - guaranteed that there's enough data.
    ///
    /// Not for direct usage.
    fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self>;

    /// Tries to unbuffer.
    ///
    /// Returns Ok(None) (and does not advance buf) if not enough data.
    fn unbuffer(buf: &mut Bytes) -> BufferResult<Option<Self>> {
        // Shallow copy
        let mut capacity_check_buf = buf.clone();
        match Self::check_capacity(capacity_check_buf) {
            CapacityCheckOutcome::NotEnoughData => Ok(None),
            CapacityCheckOutcome::Err(e) => Err(e),
            CapacityCheckOutcome::Ok(_) => Self::do_unbuffer(buf).map(|x| Some(x)), /*{
                match Self::do_unbuffer(buf) {
                    Ok(x) => Ok(Some(x)),
                    Err(e) => Err(e),
                }
            }*/
        }
    }
}

/// Implementation of size_needed for types with the ConstantBufferSize trait.
impl<T: ConstantBufferSize> BufferSize for T {
    fn size_needed(&self) -> usize {
        T::buffer_size()
    }
}

/// Implementation of check_capacity for types with the ConstantBufferSize trait.
impl<T: ConstantBufferSize> UnbufferCheckCapacity for T {
    fn check_capacity(buf: Bytes) -> CapacityCheckOutcome {
        let mut buf = buf.clone();
        let len = T::buffer_size();
        use self::CapacityCheckOutcome::*;
        if buf.len() >= len {
            buf.advance(len);
            Ok(buf)
        } else {
            NotEnoughData
        }
    }

    fn bytes_required() -> BytesRequired {
        BytesRequired::Constant(T::buffer_size())
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
            fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self> {
                let my_bytes = buf.split_to(Self::buffer_size());
                Ok(my_bytes.into_buf().$get())
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
/*
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
*/

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
/*
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
*/

pub fn check_expected(buf: &mut Bytes, expected: &'static [u8]) -> BufferResult<()> {
    let len = expected.len();
    if buf.len() < len {
        return Err(UnbufferError::BufferTooShort(buf.len(), len));
    }
    let my_bytes = buf.split_to(len);
    if my_bytes == expected {
        return Ok(());
    }
    Err(UnbufferError::UnexpectedAsciiData(
        my_bytes,
        Bytes::from_static(expected),
    ))
}
