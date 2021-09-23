// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Flag types used across VRPN.

use super::constants;
use crate::buffer_unbuffer::{buffer, unbuffer, ConstantBufferSize, WrappedConstantSize};
use bytes::{Buf, BufMut, Bytes};

bitflags! {
    /// Class of service flags matching those in the original vrpn
    pub struct ClassOfService : u32 {
        /// Results in TCP transport if available
        const RELIABLE = (1 << 0);
        const FIXED_LATENCY = (1 << 1);
        /// Results in UDP transport if available
        const LOW_LATENCY = (1 << 2);
        const FIXED_THROUGHPUT = (1 << 3);
        const HIGH_THROUGHPUT = (1 << 4);
    }
}
