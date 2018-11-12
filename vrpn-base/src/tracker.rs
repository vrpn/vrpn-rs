// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    prelude::*, MessageTypeIdentifier, Quat, Sensor, StaticTypeName, TypedMessageBody, Vec3,
};

/// Position and orientation for trackers.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseReport {
    pub sensor: Sensor,
    pub pos: Vec3,
    pub quat: Quat,
}

impl TypedMessageBody for PoseReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticTypeName(b"vrpn_Tracker Pos_Quat"));
}

/// Linear and angular velocity for trackers.
#[derive(Clone, Debug, PartialEq)]
pub struct VelocityReport {
    pub sensor: Sensor,
    pub vel: Vec3,
    pub vel_quat: Quat,
    pub vel_quat_dt: f64,
}

impl TypedMessageBody for VelocityReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticTypeName(b"vrpn_Tracker Velocity"));
}

/// Linear and angular acceleration for trackers.
#[derive(Clone, Debug, PartialEq)]
pub struct AccelReport {
    pub sensor: Sensor,
    pub acc: Vec3,
    pub acc_quat: Quat,
    pub acc_quat_dt: f64,
}

impl TypedMessageBody for AccelReport {
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier =
        MessageTypeIdentifier::UserMessageName(StaticTypeName(b"vrpn_Tracker Acceleration"));
}
