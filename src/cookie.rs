// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants::{self, COOKIE_SIZE, MAGIC_PREFIX},
    unbuffer::check_expected,
    Buffer, ConstantBufferSize, EmptyResult, Error, LogFlags, LogMode, Result, Unbuffer,
};
use bytes::{BufMut, Bytes};
use std::fmt::{self, Display, Formatter};

const COOKIE_PADDING: &[u8] = b"\0\0\0\0\0";
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

impl ConstantBufferSize for CookieData {
    #[inline]
    fn constant_buffer_size() -> usize {
        COOKIE_SIZE
    }
}

impl Buffer for CookieData {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        if buf.remaining_mut() < Self::constant_buffer_size() {
            return Err(Error::OutOfBuffer);
        }
        buf.put(self.to_string());
        buf.put(COOKIE_PADDING);
        Ok(())
    }
}

#[inline]
fn from_dec(input: &[u8]) -> Result<u8> {
    u8::from_str_radix(&String::from_utf8_lossy(input), 10).map_err(|e| Error::from(e))
}

#[inline]
fn dec_digits(buf: &mut Bytes, n: usize) -> Result<u8> {
    from_dec(&buf.split_to(n))
}

fn u8_to_log_mode(v: u8) -> LogMode {
    let mut mode = LogMode::none();
    if (v & *LogFlags::Incoming) != 0 {
        mode.set(LogFlags::Incoming);
    }
    if (v & *LogFlags::Outgoing) != 0 {
        mode.set(LogFlags::Outgoing);
    }
    mode
}

impl Unbuffer for CookieData {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<Self> {
        // remove "vrpn: ver. "
        check_expected(buf, MAGIC_PREFIX)?;

        let major: u8 = dec_digits(buf, 2)?;

        // remove dot
        check_expected(buf, b".")?;

        let minor: u8 = dec_digits(buf, 2)?;

        // remove spaces
        check_expected(buf, b"  ")?;

        let log_mode: u8 = dec_digits(buf, 1)?;
        let log_mode = u8_to_log_mode(log_mode);

        // remove padding
        check_expected(buf, COOKIE_PADDING)?;

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
        if self.is_none() {
            write!(f, "no logging")
        } else if self.is_all() {
            write!(f, "incoming and outgoing logging")
        } else if self.contains(LogFlags::Incoming) {
            write!(f, "incoming logging")
        } else if self.contains(LogFlags::Outgoing) {
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

pub fn check_ver_nonfile_compatible(ver: Version) -> EmptyResult {
    if ver.major == constants::MAGIC_DATA.major {
        Ok(())
    } else {
        Err(Error::VersionMismatch(ver, constants::MAGIC_DATA))
    }
}

pub fn check_ver_file_compatible(ver: Version) -> EmptyResult {
    if ver.major == constants::FILE_MAGIC_DATA.major {
        Ok(())
    } else {
        Err(Error::VersionMismatch(ver, constants::FILE_MAGIC_DATA))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData};
    use crate::constants::{FILE_MAGIC_DATA, MAGICLEN, MAGIC_DATA, MAGIC_PREFIX};
    use crate::prelude::*;
    use bytes::BytesMut;

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

        let mut magic_cookie = CookieData::from(MAGIC_DATA);
        magic_cookie.log_mode = Some(LogMode::none());
        assert_eq!(magic_cookie.required_buffer_size(), COOKIE_SIZE);

        let mut buf = Vec::new();
        magic_cookie
            .buffer_ref(&mut buf)
            .expect("Buffering needs to succeed");
        assert_eq!(buf.len(), COOKIE_SIZE);
    }

    #[test]
    fn ver_compat() {
        assert!(check_ver_nonfile_compatible(MAGIC_DATA).is_ok());
        assert!(check_ver_file_compatible(FILE_MAGIC_DATA).is_ok());
    }

    #[test]
    fn roundtrip() {
        let mut magic_cookie = CookieData::from(MAGIC_DATA);
        magic_cookie.log_mode = Some(LogMode::none());
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
        let mut magic_cookie = CookieData::from(MAGIC_DATA);
        magic_cookie.log_mode = Some(LogMode::none());

        let mut buf = BytesMut::new()
            .allocate_and_buffer(magic_cookie)
            .expect("Buffering needs to succeed")
            .freeze();
        assert_eq!(CookieData::unbuffer_ref(&mut buf).unwrap(), magic_cookie);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn basics() {
        assert_eq!(from_dec(b"1").unwrap(), 1_u8);
        assert_eq!(from_dec(b"12").unwrap(), 12_u8);
    }
    #[test]
    fn dec_digits_fn() {
        {
            let mut buf = Bytes::from_static(b"1");
            assert_eq!(dec_digits(&mut buf, 1).unwrap(), 1_u8);
            assert_eq!(buf.len(), 0);
        }
        {
            let mut buf = Bytes::from_static(b"12");
            assert_eq!(dec_digits(&mut buf, 2).unwrap(), 12_u8);
            assert_eq!(buf.len(), 0);
        }
    }
    #[test]
    fn parse_decimal() {
        fn parse_decimal_u8(v: &'static [u8]) -> u8 {
            from_dec(v).unwrap()
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
