// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::{Buffer, BufferResult, ConstantBufferSize, Unbuffer};
use bytes::{BufMut, Bytes};
use std::mem::size_of;
use vrpn_base::time::{Microseconds, Seconds, TimeVal};

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
    fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self> {
        let v = Unbuffer::do_unbuffer(buf)?;
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
    fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self> {
        let v: i32 = Unbuffer::do_unbuffer(buf)?;
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
    fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self> {
        let sec = Unbuffer::do_unbuffer(buf)?;
        let usec = Unbuffer::do_unbuffer(buf)?;
        Ok(TimeVal::new(sec, usec))
    }
}
