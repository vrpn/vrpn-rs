// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::{check_remaining, Buffer, BufferResult, ConstantBufferSize, Unbuffer};
use bytes::{Buf, BufMut};
use libc::timeval;
use std::mem::size_of;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Seconds(i32);

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Microseconds(i32);

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TimeVal {
    sec: Seconds,
    usec: Microseconds,
}

impl TimeVal {
    /// Constructor from components.
    /// TODO normalize
    pub fn new(sec: Seconds, usec: Microseconds) -> Self {
        Self { sec, usec }
    }

    /// Get the seconds part
    pub fn seconds(&self) -> Seconds {
        self.sec
    }

    /// Get the microseconds part
    pub fn microseconds(&self) -> Microseconds {
        self.usec
    }

    /// Get a libc timeval
    pub fn as_timeval(&self) -> timeval {
        timeval {
            tv_sec: self.sec.0 as i64,
            tv_usec: self.usec.0 as i64,
        }
    }
}

impl Default for TimeVal {
    fn default() -> Self {
        Self::new(Seconds(0), Microseconds(0))
    }
}

impl From<timeval> for TimeVal {
    fn from(v: timeval) -> Self {
        Self::new(Seconds(v.tv_sec as i32), Microseconds(v.tv_usec as i32))
    }
}
impl ConstantBufferSize for Seconds {
    fn buffer_size() -> usize {
        size_of::<Self>()
    }
}
impl Buffer for Seconds {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.0)
    }
}

impl Unbuffer for Seconds {
    fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<Self> {
        let v = Unbuffer::unbuffer(buf)?;
        Ok(Seconds(v))
    }
}

impl ConstantBufferSize for Microseconds {
    fn buffer_size() -> usize {
        size_of::<Self>()
    }
}

impl Buffer for Microseconds {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.0)
    }
}

impl Unbuffer for Microseconds {
    fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<Self> {
        let v: i32 = Unbuffer::unbuffer(buf)?;
        Ok(Microseconds(v))
    }
}

impl ConstantBufferSize for TimeVal {
    fn buffer_size() -> usize {
        Seconds::buffer_size() + Microseconds::buffer_size()
    }
}

impl Buffer for TimeVal {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.seconds());
        Buffer::buffer(buf, v.microseconds());
    }
}

impl Unbuffer for TimeVal {
    fn unbuffer<T: Buf>(buf: &mut T) -> BufferResult<Self> {
        check_remaining(buf, Self::buffer_size())?;
        let sec = Unbuffer::unbuffer(buf)?;
        let usec = Unbuffer::unbuffer(buf)?;
        Ok(TimeVal::new(sec, usec))
    }
}
