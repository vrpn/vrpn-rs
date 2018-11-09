// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use vrpn_base::{BytesRequired, EmptyResult, Error, Result};

pub mod buffer {
    use super::*;
    use bytes::{BufMut, BytesMut};

    pub trait BufMutExtras
    where
        Self: Sized,
    {
        /// Serialize the value to the buffer, without changing the allocated size.
        fn buffer<T: Buffer>(self, v: T) -> Result<Self>;
    }

    impl<U> BufMutExtras for U
    where
        U: BufMut + Sized,
    {
        fn buffer<T: Buffer>(self, v: T) -> Result<Self> {
            let mut buf = self;
            v.buffer_ref(&mut buf)?;
            Ok(buf)
        }
    }

    pub trait BytesMutExtras
    where
        Self: Sized,
    {
        /// Allocate enough space in the buffer for the given value, then serialize the value to the buffer.
        fn allocate_and_buffer<T: Buffer>(self, v: T) -> Result<Self>;
    }

    impl BytesMutExtras for BytesMut {
        fn allocate_and_buffer<T: Buffer>(mut self, v: T) -> Result<Self> {
            self.reserve(v.buffer_size());
            self.buffer(v)
        }
    }

    /// Trait for types that can be "buffered" (serialized to a byte buffer)
    pub trait Buffer: BufferSize {
        /// Serialize to a buffer (taken as a mutable reference)
        fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult;

        /// Get the number of bytes required to serialize this to a buffer.
        fn required_buffer_size(&self) -> usize {
            self.buffer_size()
        }
    }

    impl<T: WrappedConstantSize> Buffer for T {
        fn buffer_ref<U: BufMut>(&self, buf: &mut U) -> EmptyResult {
            self.get().buffer_ref(buf)
        }
    }

}

pub mod unbuffer {
    use super::*;
    use bytes::{Buf, Bytes, BytesMut, IntoBuf};

    /// Unifying trait over things we can unbuffer from (Bytes and BytesMut)
    pub trait Source:
        Sized + std::ops::Deref<Target = [u8]> + PartialEq<[u8]> + IntoBuf + Clone
    {
        fn split_to(&mut self, n: usize) -> Self;
        fn len(&self) -> usize;
        fn advance(&mut self, n: usize);
    }
    impl Source for Bytes {
        fn split_to(&mut self, n: usize) -> Self {
            Bytes::split_to(self, n)
        }
        fn len(&self) -> usize {
            Bytes::len(self)
        }
        fn advance(&mut self, n: usize) {
            Bytes::advance(self, n)
        }
    }
    impl Source for BytesMut {
        fn split_to(&mut self, n: usize) -> Self {
            BytesMut::split_to(self, n)
        }
        fn len(&self) -> usize {
            BytesMut::len(self)
        }
        fn advance(&mut self, n: usize) {
            BytesMut::advance(self, n)
        }
    }

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
        fn unbuffer_constant_size<T: Source>(buf: T) -> Result<Self>;
    }

    /// Blanket impl for types ipmlementing UnbufferConstantSize.
    impl<T: UnbufferConstantSize> Unbuffer for T {
        fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
            let len = Self::constant_buffer_size();
            if buf.len() < len {
                Err(Error::NeedMoreData(BytesRequired::Exactly(buf.len() - len)))
            } else {
                let my_buf = buf.split_to(len);
                Self::unbuffer_constant_size(my_buf)
            }
        }
    }

    impl<T: WrappedConstantSize> UnbufferConstantSize for T {
        fn unbuffer_constant_size<U: Source>(buf: U) -> Result<Self> {
            T::WrappedType::unbuffer_constant_size(buf).map(|v| T::new(v))
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

    /// Wraps a type implementing Source to allow updating of position only when a closure succeeds.
    struct SourceWrapper<'a, T: Source>(&'a mut T);
    impl<'a, T: Source> SourceWrapper<'a, T> {
        fn new(buf: &'a mut T) -> SourceWrapper<'a, T> {
            SourceWrapper(buf)
        }
        fn call<F, U, E>(self, f: F) -> std::result::Result<U, E>
        where
            F: FnOnce(&mut T) -> std::result::Result<U, E>,
        {
            let orig_len = self.0.len();
            let mut temp_buf = T::clone(self.0);
            match f(&mut temp_buf) {
                Ok(v) => {
                    self.0.advance(orig_len - temp_buf.len());
                    Ok(v)
                }
                Err(e) => Err(e),
            }
        }
    }
}

/// Trait for computing the buffer size needed for types
/// that can be "buffered" (serialized to a byte buffer),
pub trait BufferSize {
    /// Indicates the number of bytes required in the buffer to store this.
    fn buffer_size(&self) -> usize;
}

impl<T: ConstantBufferSize> BufferSize for T {
    fn buffer_size(&self) -> usize {
        T::constant_buffer_size()
    }
}

pub trait WrappedConstantSize {
    type WrappedType: buffer::Buffer + unbuffer::UnbufferConstantSize + ConstantBufferSize;
    fn get(&self) -> Self::WrappedType;
    fn new(v: Self::WrappedType) -> Self;
}

impl<T: WrappedConstantSize> ConstantBufferSize for T {
    fn constant_buffer_size() -> usize {
        T::WrappedType::constant_buffer_size()
    }
}

/// Optional trait for things that always take the same amount of space in a buffer.
///
/// Implementing this trait gets you implementations of a bunch of buffer/unbuffer-related traits for free.
pub trait ConstantBufferSize {
    /// Get the amount of space needed in a buffer.
    fn constant_buffer_size() -> usize
    where
        Self: Sized,
    {
        std::mem::size_of::<Self>()
    }
}
