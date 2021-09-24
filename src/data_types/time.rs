// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

/*!
 * Structures corresponding to time related types used by the original c++ implementation of VRPN.
 */

use crate::buffer_unbuffer::{buffer, unbuffer, ConstantBufferSize, WrappedConstantSize};

use bytes::{Buf, BufMut};
use std::{
    fmt::{Debug, Display},
    time::{Duration, SystemTime},
};

/// Structure corresponding to the C struct time_val type.
///
/// Conversions to and from native rust types are provided.
///
/// ```
/// use vrpn::data_types::TimeVal;
/// let tv = TimeVal::get_time_of_day();
/// println!("{}s, {}us since the Unix epoch", tv.seconds(), tv.microseconds());
/// println!("{}s since the Unix epoch", tv);
/// ```
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

    /// Get now as this type: equivalent to `vrpn_gettimeofday`
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

/// TimeVal is constant size
impl ConstantBufferSize for TimeVal {
    fn constant_buffer_size() -> usize {
        Seconds::constant_buffer_size() + Microseconds::constant_buffer_size()
    }
}

impl buffer::BufferTo for TimeVal {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, Self::constant_buffer_size())?;
        self.seconds().buffer_to(buf)?;
        self.microseconds().buffer_to(buf)
    }
}

impl unbuffer::UnbufferFrom for TimeVal {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        unbuffer::check_unbuffer_remaining(buf, Self::constant_buffer_size())?;
        let sec = Seconds::unbuffer_from(buf)?;
        let usec = Microseconds::unbuffer_from(buf)?;
        Ok(TimeVal::new(sec, usec))
    }
}

impl Display for TimeVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.sec, self.usec)
    }
}

/// Wrapper for an integer type for seconds
///
/// For use in `TimeVal`.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Seconds(pub i32);

/// Buffer and unbuffer seconds just like the corresponding integer
impl WrappedConstantSize for Seconds {
    type WrappedType = i32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Seconds(v)
    }
}

impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Wrapper for an integer type for microseconds.
///
/// For use in `TimeVal`.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Microseconds(pub i32);

/// Buffer and unbuffer microseconds just like the corresponding integer
impl WrappedConstantSize for Microseconds {
    type WrappedType = i32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        Microseconds(v)
    }
}

impl Display for Microseconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:06}", self.0)
    }
}
