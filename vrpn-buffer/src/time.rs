// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::Buffer;
use bytes::{BufMut, Bytes};
use size::ConstantBufferSize;
use unbuffer::{Output, Result, Unbuffer};
use vrpn_base::time::{Microseconds, Seconds, TimeVal};
use wrapped::WrappedConstantSize;

impl WrappedConstantSize for Seconds {
    type WrappedType = i32;
    fn get(self) -> Self::WrappedType {
        self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        Seconds(v)
    }
}

impl WrappedConstantSize for Microseconds {
    type WrappedType = i32;
    fn get(self) -> Self::WrappedType {
        self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        Microseconds(v)
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
