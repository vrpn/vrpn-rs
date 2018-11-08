// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{BufMut, Bytes};
use crate::{
    prelude::*,
    traits::{
        buffer::{self, Buffer},
        unbuffer::{Result as UnbufferResult, Unbuffer},
        ConstantBufferSize, WrappedConstantSize,
    },
};
use vrpn_base::time::{Microseconds, Seconds, TimeVal};

impl WrappedConstantSize for Seconds {
    type WrappedType = i32;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Seconds(v)
    }
}

impl WrappedConstantSize for Microseconds {
    type WrappedType = i32;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Microseconds(v)
    }
}

impl ConstantBufferSize for TimeVal {
    fn constant_buffer_size() -> usize {
        Seconds::constant_buffer_size() + Microseconds::constant_buffer_size()
    }
}

impl Buffer for TimeVal {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        buf.buffer(self.seconds())
            .and_then(|buf| self.microseconds().buffer_ref(buf))
    }
}

impl Unbuffer for TimeVal {
    fn unbuffer_ref(buf: &mut Bytes) -> UnbufferResult<Self> {
        Seconds::unbuffer_ref(buf)
            .and_then(|sec| Microseconds::unbuffer_ref(buf).map(|v| (v, sec)))
            .and_then(|(usec, sec)| Ok(TimeVal::new(sec, usec)))
    }
}
