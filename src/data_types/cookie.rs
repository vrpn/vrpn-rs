// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::buffer_unbuffer::{
    check_buffer_remaining, check_unbuffer_remaining, consume_expected, unbuffer_decimal_digits,
    Buffer, BufferResult, ConstantBufferSize, Unbuffer, UnbufferResult,
};

use super::{constants, LogMode};
use bytes::{Buf, BufMut};
use std::fmt::{self, Display, Formatter};

const COOKIE_PADDING: &[u8] = b"\0\0\0\0\0";

/// VRPN version number.
///
/// Only `major` matters for compatibility.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}
impl Version {
    pub fn new() -> Self {
        Self { major: 0, minor: 0 }
    }
}
impl Default for Version {
    fn default() -> Version {
        Version::new()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CookieData {
    pub version: Version,
    pub log_mode: Option<LogMode>,
}

impl CookieData {
    pub fn new() -> Self {
        Self {
            version: Version::default(),
            log_mode: None,
        }
    }

    /// Make a cookie for file use
    pub fn make_file_cookie() -> Self {
        Self::from(constants::FILE_MAGIC_DATA)
    }

    /// Make a cookie for network use
    pub fn make_cookie() -> Self {
        Self::from(constants::MAGIC_DATA)
    }
}
impl Default for CookieData {
    fn default() -> CookieData {
        CookieData::new()
    }
}

impl From<Version> for CookieData {
    fn from(version: Version) -> CookieData {
        CookieData {
            version,
            ..CookieData::default()
        }
    }
}

impl ConstantBufferSize for CookieData {
    #[inline]
    fn constant_buffer_size() -> usize {
        constants::COOKIE_SIZE
    }
}

impl Buffer for CookieData {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> BufferResult {
        check_buffer_remaining(buf, Self::constant_buffer_size())?;
        buf.put(self.to_string().as_bytes());
        buf.put(COOKIE_PADDING);
        Ok(())
    }
}

fn u8_to_log_mode(v: u8) -> LogMode {
    LogMode::from_bits_truncate(v)
}

impl Unbuffer for CookieData {
    fn unbuffer_ref<T: Buf>(buf: &mut T) -> UnbufferResult<Self> {
        check_unbuffer_remaining(buf, Self::constant_buffer_size())?;

        // remove "vrpn: ver. "
        consume_expected(buf, constants::MAGIC_PREFIX)?;

        let major: u8 = unbuffer_decimal_digits(buf, 2)?;

        // remove dot
        consume_expected(buf, b".")?;

        let minor: u8 = unbuffer_decimal_digits(buf, 2)?;

        // remove spaces
        consume_expected(buf, b"  ")?;

        let log_mode: u8 = unbuffer_decimal_digits(buf, 1)?;
        let log_mode = u8_to_log_mode(log_mode);

        // remove padding
        consume_expected(buf, COOKIE_PADDING)?;

        Ok(CookieData {
            version: Version { major, minor },
            log_mode: Some(log_mode),
        })
    }
}

impl From<CookieData> for Version {
    fn from(data: CookieData) -> Version {
        data.version
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:02}.{:02}", self.major, self.minor)
    }
}

impl Display for LogMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_empty() {
            write!(f, "no logging")
        } else if self.is_all() {
            write!(f, "incoming and outgoing logging")
        } else if self.contains(LogMode::INCOMING) {
            write!(f, "incoming logging")
        } else if self.contains(LogMode::OUTGOING) {
            write!(f, "outgoing logging")
        } else {
            write!(f, "unrecognized logging")
        }
    }
}

impl Display for CookieData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}  {}",
            String::from_utf8_lossy(constants::MAGIC_PREFIX),
            self.version,
            (self.log_mode.unwrap_or(LogMode::NONE)).bits()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionMismatch {
    actual: Version,
    expected: Version,
}

impl Display for VersionMismatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "version mismatch: expected something compatible with {}, got {}",
            self.expected, self.actual
        )
    }
}

pub fn check_ver_nonfile_compatible(ver: Version) -> Result<(), VersionMismatch> {
    if ver.major == constants::MAGIC_DATA.major {
        Ok(())
    } else {
        Err(VersionMismatch {
            actual: ver,
            expected: constants::MAGIC_DATA,
        })
    }
}

pub fn check_ver_file_compatible(ver: Version) -> Result<(), VersionMismatch> {
    if ver.major == constants::FILE_MAGIC_DATA.major {
        Ok(())
    } else {
        Err(VersionMismatch {
            actual: ver,
            expected: constants::FILE_MAGIC_DATA,
        })
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::buffer_unbuffer::BytesMutExtras;

    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(format!("{}", constants::MAGIC_DATA), "07.35");
        assert_eq!(
            format!("{}", CookieData::make_cookie()),
            "vrpn: ver. 07.35  0"
        );
        assert_eq!(
            format!("{}", CookieData::from(constants::FILE_MAGIC_DATA)),
            "vrpn: ver. 04.00  0"
        );
    }

    #[test]
    fn magic_size() {
        // Make sure the size is right.
        assert_eq!(
            constants::MAGIC_DATA.to_string().len(),
            constants::MAGICLEN - constants::MAGIC_PREFIX.len()
        );

        let mut magic_cookie = CookieData::make_cookie();
        magic_cookie.log_mode = Some(LogMode::NONE);
        assert_eq!(magic_cookie.required_buffer_size(), constants::COOKIE_SIZE);

        let mut buf = Vec::new();
        magic_cookie
            .buffer_ref(&mut buf)
            .expect("Buffering needs to succeed");
        assert_eq!(buf.len(), constants::COOKIE_SIZE);
    }

    #[test]
    fn ver_compat() {
        assert!(check_ver_nonfile_compatible(constants::MAGIC_DATA).is_ok());
        assert!(check_ver_file_compatible(constants::FILE_MAGIC_DATA).is_ok());
    }

    #[test]
    fn roundtrip() {
        let mut magic_cookie = CookieData::make_cookie();
        magic_cookie.log_mode = Some(LogMode::NONE);
        let mut buf = BytesMut::with_capacity(magic_cookie.required_buffer_size());
        magic_cookie
            .buffer_ref(&mut buf)
            .expect("Buffering needs to succeed");
        let mut buf = buf.freeze();
        assert_eq!(CookieData::unbuffer_ref(&mut buf).unwrap(), magic_cookie);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn roundtrip_bytesmut() {
        let mut magic_cookie = CookieData::make_cookie();
        magic_cookie.log_mode = Some(LogMode::INCOMING);

        let mut buf = BytesMut::new()
            .allocate_and_buffer(magic_cookie)
            .expect("Buffering needs to succeed")
            .freeze();
        assert_eq!(CookieData::unbuffer_ref(&mut buf).unwrap(), magic_cookie);
        assert_eq!(buf.len(), 0);
    }
}
