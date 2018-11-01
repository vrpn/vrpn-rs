// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{BufMut, Bytes};
use traits::{
    buffer::{self, Buffer},
    unbuffer::{Output, Result, Unbuffer},
    ConstantBufferSize, WrappedConstantSize,
};
use vrpn_base::time::{Microseconds, Seconds, TimeVal};

impl WrappedConstantSize for Seconds {
    type WrappedType = i32;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        Seconds(v)
    }
}

impl WrappedConstantSize for Microseconds {
    type WrappedType = i32;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
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
    fn buffer<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        self.seconds()
            .buffer(buf)
            .and_then(|_| self.microseconds().buffer(buf))
    }
}

impl Unbuffer for TimeVal {
    fn unbuffer(buf: &mut Bytes) -> Result<Output<Self>> {
        Seconds::unbuffer(buf)
            .and_then(|Output(sec)| Microseconds::unbuffer(buf).map(|v| (v, sec)))
            .and_then(|(Output(usec), sec)| Ok(Output(TimeVal::new(sec, usec))))
    }
}
