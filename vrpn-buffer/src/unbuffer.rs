// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, Bytes};
use itertools::join;
use nom;
use size::{BytesRequired, ConstantBufferSize};
use std::num::ParseIntError;

quick_error!{
#[derive(Debug)]
pub enum Error {
    NeedMoreData(needed: BytesRequired, buf: Bytes) {
        display("ran out of buffered bytes: need {}", needed)
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
    ParseError(msg: String) {
        description(msg)
        from(err: nom::Err<&[u8]>) -> (err.to_string())
    }
}
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Output<T>(pub Bytes, pub T);

impl<T> Output<T> {
    pub fn new(remaining: Bytes, data: T) -> Output<T> {
        Output(remaining, data)
    }
    pub fn from_slice<'a>(containing_buf: Bytes, remaining: &'a [u8], data: T) -> Output<T> {
        Self::new(containing_buf.slice_ref(remaining), data)
    }
    pub fn to_child_remaining<'a>(&self, remaining: &'a [u8], data: T) -> Output<T> {
        Output::from_slice(self.0.clone(), remaining, data)
    }

    pub fn remaining(&self) -> Bytes {
        self.0.clone()
    }

    pub fn data(self) -> T {
        self.1
    }

    pub fn borrow_data<'a>(&'a self) -> &'a T {
        &self.1
    }

    pub fn borrow_data_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.1
    }

    pub fn map<U, F>(self, f: F) -> Output<U>
    where
        U: Sized,
        F: FnOnce(T) -> U,
    {
        Output::new(self.0, f(self.1))
    }
}

impl<T> Default for Output<T>
where
    T: Default,
{
    fn default() -> Output<T> {
        Output(Bytes::default(), T::default())
    }
}

/// Trait for types that can be "unbuffered" (parsed from a byte buffer)
pub trait Unbuffer: Sized {
    /// Tries to unbuffer.
    ///
    /// Returns Ok(None) if not enough data.
    fn unbuffer(buf: Bytes) -> Result<Output<Self>>;
}

/// Implementation trait for constant-buffer-size types,
/// used by the blanket implementation of Unbuffer.
pub trait UnbufferConstantSize: Sized + ConstantBufferSize {
    /// Perform the unbuffering: only called with at least as many bytes as needed.
    fn unbuffer_constant_size(buf: Bytes) -> Result<Self>;
}

/// Blanket impl for types ipmlementing UnbufferConstantSize.
impl<T: UnbufferConstantSize> Unbuffer for T {
    fn unbuffer(buf: Bytes) -> Result<Output<Self>> {
        let len = Self::constant_buffer_size();
        if buf.len() < len {
            Err(Error::NeedMoreData(
                BytesRequired::Exactly(buf.len() - len),
                buf,
            ))
        } else {
            let mut remaining = buf.clone();
            let my_buf = remaining.split_to(len);
            match Self::unbuffer_constant_size(my_buf) {
                Ok(v) => Ok(Output::new(remaining, v)),
                Err(e) => Err(e),
            }
        }
    }
}

pub trait AndThenMap<T> {
    fn and_then_map<U, F>(self, f: F) -> Result<Output<U>>
    where
        U: Sized,
        F: FnOnce(T) -> U;
}

impl<T> AndThenMap<T> for Result<Output<T>> {
    /// Transforms the completed output's data, if successful
    fn and_then_map<U, F>(self, f: F) -> Result<Output<U>>
    where
        U: Sized,
        F: FnOnce(T) -> U,
    {
        match self {
            Ok(Output(remaining, v)) => Ok(Output(remaining, f(v))),
            Err(e) => Err(e),
        }
    }
}
