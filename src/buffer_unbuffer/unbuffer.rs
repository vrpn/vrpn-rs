// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Traits, etc. related to unbuffering types

use std::num::ParseIntError;

use super::{BufferUnbufferError, ConstantBufferSize, SizeRequirement, WrappedConstantSize};
use bytes::{Buf, Bytes};

pub type UnbufferResult<T> = std::result::Result<T, BufferUnbufferError>;

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait UnbufferFrom: Sized {
    /// Tries to unbuffer, advancing the buffer position only if successful.
    ///
    /// # Note
    ///
    /// Must check size before advancing the buffer: usually start with
    /// `check_unbuffer_remaining`, may require use of `peek_u32`
    ///
    /// Returns `Err(BufferUnbufferError::NeedMoreData(n))` if not enough data.
    fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self>;
}

/// Tries to unbuffer from a mutable reference to a buffer.
///
/// Delegates to `UnbufferFrom::unbuffer_from()`.
/// Returns `Err(BufferUnbufferError::NeedMoreData(n))` if not enough data.
#[deprecated = "Use UnbufferFrom::unbuffer_from() directly instead"]
pub fn unbuffer_from<T: UnbufferFrom, U: Buf>(buf: &mut U) -> UnbufferResult<T> {
    T::unbuffer_from(buf)
}

/// Tries to unbuffer, consuming the buffer and returning what's left.
///
/// Should no longer be neccessary now that futures don't require you to consume and return streams
/// with every call.
///
/// Returns `Err(BufferUnbufferError::NeedMoreData(n))` if not enough data.
#[deprecated = "Should not be necessary with modern futures, use UnbufferFrom::unbuffer_from() directly instead"]
pub fn unbuffer_from_and_into<T: UnbufferFrom>(buf: Bytes) -> UnbufferResult<(T, Bytes)> {
    let mut buf = buf;
    let v = T::unbuffer_from(&mut buf)?;
    Ok((v, buf))
}

/// Implementation trait for constant-buffer-size types,
/// used by the blanket implementation of `UnbufferFrom`.
pub trait UnbufferConstantSize: Sized + ConstantBufferSize {
    /// Perform the unbuffering: only called with at least as many bytes as needed.
    /// Therefore, no need to size-check.
    fn unbuffer_constant_size<T: Buf>(buf: &mut T) -> UnbufferResult<Self>;
}

/// Blanket impl for types implementing UnbufferConstantSize.
// TODO implement unbuffer_constant_size everywhere we're checking remaining against Self::constant_buffer_size
impl<T: UnbufferConstantSize> UnbufferFrom for T {
    fn unbuffer_from<U: Buf>(buf: &mut U) -> UnbufferResult<Self> {
        let len = Self::constant_buffer_size();
        check_unbuffer_remaining(buf, len)?;
        let mut buf_subset = buf.take(len);
        let mut bytes_subset = buf_subset.copy_to_bytes(len);
        let result = Self::unbuffer_constant_size(&mut bytes_subset);
        // don't advance if we need more data
        if let Err(BufferUnbufferError::NeedMoreData(n)) = result {
            return Err(BufferUnbufferError::NeedMoreData(n));
        }
        buf.advance(len);
        result
    }
}

impl<T: WrappedConstantSize> UnbufferConstantSize for T {
    fn unbuffer_constant_size<U: Buf>(buf: &mut U) -> UnbufferResult<Self> {
        T::WrappedType::unbuffer_constant_size(buf).map(T::new)
    }
}

/// Check whether a buffer has enough bytes remaining to unbuffer a given length
pub fn check_unbuffer_remaining<T: Buf>(
    buf: &T,
    required_len: usize,
) -> std::result::Result<(), BufferUnbufferError> {
    let bytes_len = buf.remaining();
    if bytes_len < required_len {
        Err(SizeRequirement::Exactly(required_len - bytes_len).into())
    } else {
        Ok(())
    }
}

/// Consume the expected static byte string from the buffer.
///
/// ```
/// use vrpn::buffer_unbuffer::consume_expected;
/// use bytes::Buf;
/// let mut buf = &b"hello world"[..];
/// assert_eq!(buf.remaining(), 11);
/// assert!(consume_expected(&mut buf, &b"hello"[..]).is_ok());
/// assert_eq!(buf.remaining(), 6);
/// ```
pub fn consume_expected<T: Buf>(
    buf: &mut T,
    expected: &'static [u8],
) -> std::result::Result<(), BufferUnbufferError> {
    let expected_len = expected.len();
    check_unbuffer_remaining(buf, expected_len)?;

    let my_bytes = buf.copy_to_bytes(expected_len);
    if my_bytes == expected {
        Ok(())
    } else {
        Err(BufferUnbufferError::UnexpectedAsciiData {
            actual: my_bytes,
            expected: Bytes::from_static(expected),
        })
    }
}

/// Peek at a leading u32 without advancing the buffer.
///
/// ```
/// use vrpn::buffer_unbuffer::peek_u32;
/// use bytes::{Buf, Bytes};
/// let data = b"\0\0\0\0";
/// let mut buf = Bytes::copy_from_slice(&data[..]);
/// assert_eq!(peek_u32(&buf).unwrap(), 0);
/// assert_eq!(buf.remaining(), 4);
/// ```
pub fn peek_u32<T: Buf>(buf: &T) -> Option<u32> {
    const SIZE_LEN: usize = std::mem::size_of::<u32>();
    if buf.remaining() < SIZE_LEN {
        eprintln!("Not enough remaining bytes for the size.");
        return None;
    }
    let mut chunk = buf.chunk();
    if chunk.len() < SIZE_LEN {
        eprintln!("Not enough remaining bytes in the chunk for the size.");
        // Some(buf.clone().get_u32())
        None
    } else {
        Some(chunk.get_u32())
    }
}

#[inline]
fn from_dec(input: Bytes) -> std::result::Result<u8, ParseIntError> {
    str::parse::<u8>(&String::from_utf8_lossy(&input))
}

#[inline]
pub fn unbuffer_decimal_digits<T: Buf>(buf: &mut T, n: usize) -> UnbufferResult<u8> {
    let val = from_dec(buf.copy_to_bytes(n))?;

    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basics() {
        assert_eq!(from_dec(Bytes::from_static(b"1")).unwrap(), 1_u8);
        assert_eq!(from_dec(Bytes::from_static(b"12")).unwrap(), 12_u8);
    }
    #[test]
    fn dec_digits_fn() {
        {
            let mut buf = Bytes::from_static(b"1");
            assert_eq!(unbuffer_decimal_digits(&mut buf, 1).unwrap(), 1_u8);
            assert_eq!(buf.len(), 0);
        }
        {
            let mut buf = Bytes::from_static(b"12");
            assert_eq!(unbuffer_decimal_digits(&mut buf, 2).unwrap(), 12_u8);
            assert_eq!(buf.len(), 0);
        }
    }
    #[test]
    fn parse_decimal() {
        fn parse_decimal_u8(v: &'static [u8]) -> u8 {
            let myval = Bytes::from_static(v);
            from_dec(myval).unwrap()
        }
        assert_eq!(0_u8, parse_decimal_u8(b"0"));
        assert_eq!(0_u8, parse_decimal_u8(b"00"));
        assert_eq!(0_u8, parse_decimal_u8(b"000"));
        assert_eq!(1_u8, parse_decimal_u8(b"1"));
        assert_eq!(1_u8, parse_decimal_u8(b"01"));
        assert_eq!(1_u8, parse_decimal_u8(b"001"));
        assert_eq!(1_u8, parse_decimal_u8(b"0001"));
        assert_eq!(10_u8, parse_decimal_u8(b"10"));
        assert_eq!(10_u8, parse_decimal_u8(b"010"));
        assert_eq!(10_u8, parse_decimal_u8(b"0010"));
        assert_eq!(10_u8, parse_decimal_u8(b"00010"));
    }
}
