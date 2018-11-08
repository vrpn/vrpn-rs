// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants::{self, MAGIC_PREFIX},
    log::{LogFlags, LogMode},
};
use std::{
    fmt::{self, Display, Formatter},
    result,
};

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

impl Display for LogMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_none() {
            write!(f, "no logging")
        } else if self.is_all() {
            write!(f, "incoming and outgoing logging")
        } else if self.contains(LogFlags::INCOMING) {
            write!(f, "incoming logging")
        } else if self.contains(LogFlags::OUTGOING) {
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
            String::from_utf8_lossy(&MAGIC_PREFIX[..]),
            self.version,
            *(self.log_mode.unwrap_or(LogMode::none()))
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
    use super::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData};
    use crate::constants::{FILE_MAGIC_DATA, MAGICLEN, MAGIC_DATA, MAGIC_PREFIX};
    #[test]
    fn formatting() {
        assert_eq!(format!("{}", MAGIC_DATA), "07.35");
        assert_eq!(
            format!("{}", CookieData::from(MAGIC_DATA)),
            "vrpn: ver. 07.35  0"
        );
        assert_eq!(
            format!("{}", CookieData::from(FILE_MAGIC_DATA)),
            "vrpn: ver. 04.00  0"
        );
    }

    #[test]
    fn magic_size() {
        // Make sure the size is right.
        assert_eq!(MAGIC_DATA.to_string().len(), MAGICLEN - MAGIC_PREFIX.len());
    }

    #[test]
    fn ver_compat() {
        assert!(check_ver_nonfile_compatible(MAGIC_DATA).is_ok());
        assert!(check_ver_file_compatible(FILE_MAGIC_DATA).is_ok());
    }
}
