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
    /// Tries to unbuffer.
    ///
    /// # Errors
    /// In case of error, your buffer might be at any place (advanced an arbitrary number of bytes).
    /// If this bothers you, give us a clone of your buffer.
    fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self>;
}

/// Implementation trait for constant-buffer-size types,
/// used by the blanket implementation of `UnbufferFrom`.
#[deprecated]
pub trait UnbufferConstantSize: Sized + ConstantBufferSize {
    /// Perform the unbuffering: only called with at least as many bytes as needed.
    /// Therefore, no need to size-check.
    fn unbuffer_constant_size<T: Buf>(buf: &mut T) -> UnbufferResult<Self>;
}

/// Blanket impl for types implementing WrappedConstantSize.
impl<T: WrappedConstantSize> UnbufferFrom for T {
    fn unbuffer_from<U: Buf>(buf: &mut U) -> UnbufferResult<Self> {
        T::WrappedType::unbuffer_from(buf).map(T::new)
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

/// Structs implementing Buf that we know how to peek_u32 on.
pub trait PeekU32: Buf {
    /// Peek at a leading u32 without advancing the buffer.
    fn peek_u32(&self) -> Option<u32>;
}

impl PeekU32 for Bytes {
    fn peek_u32(&self) -> Option<u32> {
        let size_len: usize = u32::constant_buffer_size();
        if self.len() < size_len {
            return None;
        }
        let peeked = Bytes::copy_from_slice(&self[..size_len]).get_u32();
        Some(peeked)
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
    fn peek() {
        use bytes::Buf;
        let data = b"\0\0\0\0";
        {
            let buf = &data[..];
            assert_eq!(peek_u32(&buf), Some(0));
            assert_eq!(buf.remaining(), data.len());
        }
    }
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
