// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::Buffer;
use bytes::{BufMut, Bytes};
use size::ConstantBufferSize;
use std::mem::size_of;
use unbuffer::{Output, Result, Unbuffer, UnbufferConstantSize};
use vrpn_base::time::{Microseconds, Seconds, TimeVal};

impl ConstantBufferSize for Seconds {
    fn constant_buffer_size() -> usize {
        size_of::<Self>()
    }
}

impl Buffer for Seconds {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.0)
    }
}

impl UnbufferConstantSize for Seconds {
    fn unbuffer_constant_size(buf: Bytes) -> Result<Self> {
        i32::unbuffer_constant_size(buf).map(|v| Seconds(v))
    }
}

impl ConstantBufferSize for Microseconds {
    fn constant_buffer_size() -> usize {
        size_of::<Self>()
    }
}

impl Buffer for Microseconds {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.0)
    }
}

impl UnbufferConstantSize for Microseconds {
    fn unbuffer_constant_size(buf: Bytes) -> Result<Self> {
        i32::unbuffer_constant_size(buf).map(|v| Microseconds(v))
    }
}

impl ConstantBufferSize for TimeVal {
    fn constant_buffer_size() -> usize {
        Seconds::constant_buffer_size() + Microseconds::constant_buffer_size()
    }
}

impl Buffer for TimeVal {
    fn buffer<T: BufMut>(buf: &mut T, v: Self) {
        Buffer::buffer(buf, v.seconds());
        Buffer::buffer(buf, v.microseconds());
    }
}

impl Unbuffer for TimeVal {
    fn unbuffer(buf: Bytes) -> Result<Output<Self>> {
        Seconds::unbuffer(buf)
            .and_then(|Output(remaining, sec)| Microseconds::unbuffer(remaining).map(|v| (v, sec)))
            .and_then(|(Output(remaining, usec), sec)| {
                Ok(Output(remaining, TimeVal::new(sec, usec)))
            })
    }
}
