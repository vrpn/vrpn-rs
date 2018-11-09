// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{BufMut, Bytes};
use crate::{prelude::*, Buffer, ConstantBufferSize, Unbuffer};
use vrpn_base::{error::*, tracker::*, Quat, Sensor, Vec3};

impl ConstantBufferSize for PoseReport {
    fn constant_buffer_size() -> usize {
        Sensor::constant_buffer_size() * 2
            + Vec3::constant_buffer_size()
            + Quat::constant_buffer_size()
    }
}

impl Buffer for PoseReport {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        self.sensor.buffer_ref(buf)?;
        // padding
        self.sensor.buffer_ref(buf)?;
        self.pos.buffer_ref(buf)?;
        self.quat.buffer_ref(buf)?;
        Ok(())
    }
}

impl Unbuffer for PoseReport {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
        let sensor = Sensor::unbuffer_ref(buf)?;
        let _ = Sensor::unbuffer_ref(buf)?;
        let pos = Vec3::unbuffer_ref(buf)?;
        let quat = Quat::unbuffer_ref(buf)?;
        Ok(PoseReport { sensor, pos, quat })
    }
}
