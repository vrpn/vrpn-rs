// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{BytesRequired, ConstantBufferSize, Error, Result, WrappedConstantSize};
use bytes::{Buf, Bytes, BytesMut};

/// Unifying trait over things we can unbuffer from (Bytes and BytesMut)
pub trait Source: Sized + std::ops::Deref<Target = [u8]> + PartialEq<[u8]> + Clone {
    fn split_to(&mut self, n: usize) -> Self;
    fn len(&self) -> usize;
    fn advance(&mut self, n: usize);
    fn is_empty(&self) -> bool;
}

impl Source for Bytes {
    fn split_to(&mut self, n: usize) -> Self {
        Bytes::split_to(self, n)
    }
    fn len(&self) -> usize {
        Bytes::len(self)
    }
    fn advance(&mut self, n: usize) {
        Buf::advance(self, n)
    }
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// impl Source for BytesMut {
//     fn split_to(&mut self, n: usize) -> Self {
//         BytesMut::split_to(self, n)
//     }
//     fn len(&self) -> usize {
//         BytesMut::len(self)
//     }
//     fn advance(&mut self, n: usize) {
//         BytesMut::advance(self, n)
//     }
//     fn is_empty(&self) -> bool {
//         self.is_empty()
//     }
// }

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait Unbuffer: Sized {
    /// Tries to unbuffer.
    ///
    /// Returns Err(Error::NeedMoreData(n)) if not enough data.
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self>;
}

/// Tries to unbuffer from a mutable reference to a buffer.
///
/// Delegates to Unbuffer::unbuffer_ref().
/// Returns Err(Error::NeedMoreData(n)) if not enough data.
pub fn unbuffer_ref<T: Unbuffer>(buf: &mut Bytes) -> Result<T> {
    T::unbuffer_ref(buf)
}

/// Tries to unbuffer.
///
/// Returns Err(Error::NeedMoreData(n)) if not enough data.
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
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
        let len = Self::constant_buffer_size();
        if buf.len() < len {
            Err(Error::NeedMoreData(BytesRequired::Exactly(buf.len() - len)))
        } else {
            Self::unbuffer_constant_size(&mut buf.split_to(len))
        }
    }
}

impl<T: WrappedConstantSize> UnbufferConstantSize for T {
    fn unbuffer_constant_size<U: Buf>(buf: &mut U) -> Result<Self> {
        T::WrappedType::unbuffer_constant_size(buf).map(T::new)
    }
}

/// Trait used to extend the methods of Result<Output<T>>
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
pub trait UnbufferOutput {}
impl<T> UnbufferOutput for T where T: Unbuffer {}
impl<T, U> UnbufferOutput for (T, U) where T: Unbuffer {}

impl<T: UnbufferOutput> OutputResultExtras<T> for Result<T> {
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

pub trait BytesExtras
where
    Self: Source,
{
    fn unbuffer<T>(self) -> Result<(T, Self)>
    where
        T: Unbuffer;
}
impl BytesExtras for Bytes {
    fn unbuffer<T>(self) -> Result<(T, Self)>
    where
        T: Unbuffer,
    {
        let mut buf = self;
        let v = T::unbuffer_ref(&mut buf)?;
        Ok((v, buf))
    }
}

/// Check that the buffer begins with the expected string.
pub fn check_expected(buf: &mut Bytes, expected: &'static [u8]) -> Result<()> {
    let bytes_len = buf.len();
    let expected_len = expected.len();
    if bytes_len < expected_len {
        return Err(Error::NeedMoreData(BytesRequired::Exactly(
            expected_len - bytes_len,
        )));
    }
    let my_bytes = buf.split_to(expected_len);
    if my_bytes == expected {
        Ok(())
    } else {
        Err(Error::UnexpectedAsciiData(
            my_bytes,
            Bytes::from_static(expected),
        ))
    }
}
