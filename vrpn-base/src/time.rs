// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use libc::timeval;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Seconds(pub i32);

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Microseconds(pub i32);

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct TimeVal {
    sec: Seconds,
    usec: Microseconds,
}

impl TimeVal {
    /// Constructor from components.
    /// TODO normalize
    pub fn new(sec: Seconds, usec: Microseconds) -> Self {
        Self { sec, usec }
    }

    /// Get the seconds part
    pub fn seconds(&self) -> Seconds {
        self.sec
    }

    /// Get the microseconds part
    pub fn microseconds(&self) -> Microseconds {
        self.usec
    }

    /// Get a libc timeval
    pub fn as_timeval(&self) -> timeval {
        timeval {
            tv_sec: self.sec.0 as i64,
            tv_usec: self.usec.0 as i64,
        }
    }
}

impl Default for TimeVal {
    fn default() -> Self {
        Self::new(Seconds(0), Microseconds(0))
    }
}

impl From<timeval> for TimeVal {
    fn from(v: timeval) -> Self {
        Self::new(Seconds(v.tv_sec as i32), Microseconds(v.tv_usec as i32))
    }
}
