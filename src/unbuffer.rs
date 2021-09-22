// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Traits, etc. related to unbuffering types

use crate::{BytesRequired, ConstantBufferSize, Error, Result, WrappedConstantSize};
use bytes::{Buf, Bytes};

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait Unbuffer: Sized {
    /// Tries to unbuffer, advancing the buffer position only if successful.
    ///
    /// Returns `Err(Error::NeedMoreData(n))` if not enough data.
    fn unbuffer_ref<T: Buf>(buf: &mut T) -> Result<Self>;
}

/// Tries to unbuffer from a mutable reference to a buffer.
///
/// Delegates to `Unbuffer::unbuffer_ref()`.
/// Returns `Err(Error::NeedMoreData(n))` if not enough data.
#[deprecated]
pub fn unbuffer_ref<T: Unbuffer, U: Buf>(buf: &mut U) -> Result<T> {
    T::unbuffer_ref(buf)
}

/// Tries to unbuffer, consuming the buffer and returning what's left.
///
/// Should no longer be neccessary now that futures don't require you to consume and return streams
/// with every call.
///
/// Returns `Err(Error::NeedMoreData(n))` if not enough data.
#[deprecated]
pub fn unbuffer_from<T: Unbuffer>(buf: Bytes) -> Result<(T, Bytes)> {
    let mut buf = buf;
    let v = T::unbuffer_ref(&mut buf)?;
    Ok((v, buf))
}

/// Implementation trait for constant-buffer-size types,
/// used by the blanket implementation of Unbuffer.
pub trait UnbufferConstantSize: Sized + ConstantBufferSize {
    /// Perform the unbuffering: only called with at least as many bytes as needed.
    fn unbuffer_constant_size<T: Buf>(buf: &mut T) -> Result<Self>;
}

/// Blanket impl for types implementing UnbufferConstantSize.
impl<T: UnbufferConstantSize> Unbuffer for T {
    fn unbuffer_ref<U: Buf>(buf: &mut U) -> Result<Self> {
        let len = Self::constant_buffer_size();
        if buf.remaining() < len {
            Err(Error::NeedMoreData(BytesRequired::Exactly(
                buf.remaining() - len,
            )))
        } else {
            let mut buf_subset = buf.take(len);
            let mut bytes_subset = buf_subset.copy_to_bytes(len);
            let result = Self::unbuffer_constant_size(&mut bytes_subset);
            // don't advance if we need more data
            if let Err(Error::NeedMoreData(n)) = result {
                return Err(Error::NeedMoreData(n));
            }
            buf.advance(len);
            result
        }
    }
}

impl<T: WrappedConstantSize> UnbufferConstantSize for T {
    fn unbuffer_constant_size<U: Buf>(buf: &mut U) -> Result<Self> {
        T::WrappedType::unbuffer_constant_size(buf).map(T::new)
    }
}

/// Extension trait used to extend the methods of Result<Output<T>>
pub trait OutputResultExtras<T> {
    /// Map the result that "exactly" n additional bytes are
    /// required to "at least" n additional bytes are required.
    ///
    /// Used when a variable-buffer-size type begins its work by
    /// unbuffering a fixed-size type, like a "length" field.
    fn map_exactly_err_to_at_least(self) -> Self;

    /// Map the result that additional bytes are required to a
    /// generic parse error with the byte count in the message,
    /// for instances where more bytes are logically unavailable.
    fn map_need_more_err_to_generic_parse_err(self, task: &str) -> Self;
}

/// A trait identifying a structure that can be unbuffered, or a pair of something that can be unbuffered and something else.
///
/// These are types that may be returned in a Result<> by an unbuffer operation.
/// Used to automatically select which Result<> get the `OutputResultExtras<T>` extension trait.
pub trait UnbufferOutput {}
impl<T> UnbufferOutput for T where T: Unbuffer {}
impl<T, U> UnbufferOutput for (T, U) where T: Unbuffer {}

impl<T: UnbufferOutput> OutputResultExtras<T> for Result<T> {
    /// Convert an error that you need exactly some amount of bytes, into the error that you need at least that much.
    fn map_exactly_err_to_at_least(self) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(Error::NeedMoreData(BytesRequired::Exactly(n))) => {
                Err(Error::NeedMoreData(BytesRequired::AtLeast(n)))
            }
            Err(e) => Err(e),
        }
    }

    fn map_need_more_err_to_generic_parse_err(self, task: &str) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(Error::NeedMoreData(n)) => Err(Error::OtherMessage(format!(
                "when {}, ran out of data - needed {} additional bytes",
                task, n
            ))),
            Err(e) => Err(e),
        }
    }
}

// pub trait BytesExtras
// where
//     Self: Source,
// {
//     fn unbuffer<T>(self) -> Result<(T, Self)>
//     where
//         T: Unbuffer;
// }
// impl BytesExtras for Bytes {
//     fn unbuffer<T>(self) -> Result<(T, Self)>
//     where
//         T: Unbuffer,
//     {
//         let mut buf = self;
//         let v = T::unbuffer_ref(&mut buf)?;
//         Ok((v, buf))
//     }
// }

/// Consume the expected static byte string from the buffer.
///
/// ```
/// use vrpn::unbuffer::consume_expected;
/// use bytes::Buf;
/// let mut buf = &b"hello world"[..];
/// assert_eq!(buf.remaining(), 11);
/// assert!(consume_expected(&mut buf, &b"hello"[..]).is_ok());
/// assert_eq!(buf.remaining(), 6);
/// ```
pub fn consume_expected<T: Buf>(buf: &mut T, expected: &'static [u8]) -> Result<()> {
    let bytes_len = buf.remaining();
    let expected_len = expected.len();
    if bytes_len < expected_len {
        return Err(Error::NeedMoreData(BytesRequired::Exactly(
            expected_len - bytes_len,
        )));
    }

    let my_bytes = buf.copy_to_bytes(expected_len);
    if my_bytes == expected {
        Ok(())
    } else {
        Err(Error::UnexpectedAsciiData(
            my_bytes,
            Bytes::from_static(expected),
        ))
    }
}
