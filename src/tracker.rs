// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Types related to the `vrpn_Tracker` device class

use crate::{
    buffer_unbuffer::{
        buffer::{check_buffer_remaining, BufferResult, BufferTo},
        unbuffer::{check_unbuffer_remaining, UnbufferFrom, UnbufferResult},
        ConstantBufferSize,
    },
    data_types::{
        id_types::Sensor,
        message::{MessageTypeIdentifier, TypedMessageBody},
        name_types::StaticMessageTypeName,
        Quat, Vec3,
    },
};
use bytes::{Buf, BufMut};

/// Position and orientation for trackers.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseReport {
    /// Sensor id
    pub sensor: Sensor,
    /// Position
    pub pos: Vec3,
    /// Orientation
    pub quat: Quat,
}

impl TypedMessageBody for PoseReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticMessageTypeName(b"vrpn_Tracker Pos_Quat"));
}

impl ConstantBufferSize for PoseReport {
    fn constant_buffer_size() -> usize {
        Sensor::constant_buffer_size() * 2
            + Vec3::constant_buffer_size()
            + Quat::constant_buffer_size()
    }
}

impl BufferTo for PoseReport {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.sensor.buffer_to(buf)?;
        // padding
        self.sensor.buffer_to(buf)?;
        self.pos.buffer_to(buf)?;
        self.quat.buffer_to(buf)?;
        Ok(())
    }
}

impl UnbufferFrom for PoseReport {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let sensor = Sensor::unbuffer_from(buf)?;
        let _ = Sensor::unbuffer_from(buf)?;
        let pos = Vec3::unbuffer_from(buf)?;
        let quat = Quat::unbuffer_from(buf)?;
        Ok(PoseReport { sensor, pos, quat })
    }
}

/// Linear and angular velocity for trackers.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VelocityReport {
    pub sensor: Sensor,
    pub vel: Vec3,
    pub vel_quat: Quat,
    pub vel_quat_dt: f64,
}

impl TypedMessageBody for VelocityReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticMessageTypeName(b"vrpn_Tracker Velocity"));
}

/// Linear and angular acceleration for trackers.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AccelReport {
    pub sensor: Sensor,
    pub acc: Vec3,
    pub acc_quat: Quat,
    pub acc_quat_dt: f64,
}

impl TypedMessageBody for AccelReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticMessageTypeName(b"vrpn_Tracker Acceleration"));
}
