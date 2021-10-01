// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>


impl From<cgmath::Vector3<f64>> for Vec3 {
    fn from(v: cgmath::Vector3<f64>) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for cgmath::Vector3<f64> {
    fn from(v: Vec3) -> Self {
        cgmath::Vector3::new(v.x, v.y, v.z)
    }
}

impl From<cgmath::Quaternion<f64>> for Quat {
    fn from(q: cgmath::Quaternion<f64>) -> Self {
        Quat {
            s: q.s,
            v: q.v.into(),
        }
    }
}

impl From<Quat> for cgmath::Quaternion<f64> {
    fn from(q: Quat) -> Self {
        cgmath::Quaternion::from_sv(q.s, q.v.into())
    }
}
