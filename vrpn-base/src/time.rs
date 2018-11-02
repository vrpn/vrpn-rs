// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::time::{Duration, SystemTime};

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
    ///
    /// TODO normalize?
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

    pub fn get_time_of_day() -> TimeVal {
        TimeVal::from(SystemTime::now())
    }
}

impl Default for TimeVal {
    fn default() -> Self {
        Self::new(Seconds(0), Microseconds(0))
    }
}

impl From<SystemTime> for TimeVal {
    fn from(v: SystemTime) -> Self {
        // In practice this should always work.
        let since_epoch = v.duration_since(SystemTime::UNIX_EPOCH).unwrap();

        TimeVal::new(
            Seconds(since_epoch.as_secs() as i32),
            Microseconds(since_epoch.subsec_micros() as i32),
        )
    }
}

impl From<TimeVal> for SystemTime {
    fn from(v: TimeVal) -> Self {
        SystemTime::UNIX_EPOCH
            + Duration::from_secs(v.seconds().0 as u64)
            + Duration::from_micros(v.microseconds().0 as u64)
    }
}
