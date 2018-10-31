// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use constants::{self, MAGIC_PREFIX};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::result;

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
    pub log_mode: Option<u8>,
}

impl CookieData {
    pub fn new() -> Self {
        Self {
            version: Version::default(),
            log_mode: None,
        }
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

impl Display for CookieData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}  {}",
            String::from_utf8_lossy(&MAGIC_PREFIX[..]),
            self.version,
            self.log_mode.unwrap_or(0)
        )
    }
}

quick_error!{
#[derive(Debug)]
pub enum VersionError {
    VersionMismatch(actual: Version, expected: Version) {
        display(
                "version mismatch: expected something compatible with {}, got {}",
                expected, actual)
    }
}
}
pub fn check_ver_nonfile_compatible(ver: Version) -> result::Result<(), VersionError> {
    if ver.major == constants::MAGIC_DATA.major {
        Ok(())
    } else {
        Err(VersionError::VersionMismatch(ver, constants::MAGIC_DATA))
    }
}

pub fn check_ver_file_compatible(ver: Version) -> result::Result<(), VersionError> {
    if ver.major == constants::FILE_MAGIC_DATA.major {
        Ok(())
    } else {
        Err(VersionError::VersionMismatch(
            ver,
            constants::FILE_MAGIC_DATA,
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formatting() {
        assert_eq!(format!("{}", super::constants::MAGIC_DATA), "07.35");
        assert_eq!(
            format!("{}", super::CookieData::from(super::constants::MAGIC_DATA)),
            "vrpn: ver. 07.35  0"
        );
        assert_eq!(
            format!(
                "{}",
                super::CookieData::from(super::constants::FILE_MAGIC_DATA)
            ),
            "vrpn: ver. 04.00  0"
        );
    }
    #[test]
    fn ver_compat() {
        assert!(super::check_ver_nonfile_compatible(super::constants::MAGIC_DATA).is_ok());
        assert!(super::check_ver_file_compatible(super::constants::FILE_MAGIC_DATA).is_ok());
    }
    #[test]
    fn parse_decimal() {
        fn parse_decimal_u8(v: &'static [u8]) -> u8 {
            super::parse_decimal_u8(::bytes::Bytes::from_static(v)).unwrap()
        }
        assert_eq!(0_u8, parse_decimal_u8(b"0"));
        assert_eq!(0_u8, parse_decimal_u8(b"00"));
        assert_eq!(0_u8, parse_decimal_u8(b"000"));
        assert_eq!(1_u8, parse_decimal_u8(b"1"));
        assert_eq!(1_u8, parse_decimal_u8(b"01"));
        assert_eq!(1_u8, parse_decimal_u8(b"001"));
        assert_eq!(1_u8, parse_decimal_u8(b"0001"));
        assert_eq!(10_u8, parse_decimal_u8(b"10"));
        assert_eq!(10_u8, parse_decimal_u8(b"010"));
        assert_eq!(10_u8, parse_decimal_u8(b"0010"));
        assert_eq!(10_u8, parse_decimal_u8(b"00010"));
    }
}
