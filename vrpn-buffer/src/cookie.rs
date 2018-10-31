// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::{check_expected, Buffer, BufferResult, ConstantBufferSize, Unbuffer, UnbufferError};
use bytes::{Buf, BufMut, Bytes, IntoBuf};
use std::fmt::{self, Display, Formatter};
use std::result;
use vrpn_base::constants::{self, COOKIE_SIZE, MAGIC_PREFIX};
use vrpn_base::cookie::{CookieData, Version};

impl ConstantBufferSize for CookieData {
    fn buffer_size() -> usize {
        COOKIE_SIZE
    }
}

impl Buffer for CookieData {
    fn buffer<T: BufMut>(buf: &mut T, v: CookieData) {
        let s = format!("{}", v);
        let padding = COOKIE_SIZE - s.len();
        buf.put(s);
        for _ in 0..padding {
            buf.put_u8(0);
        }
    }
}

fn parse_decimal_u8<T: IntoBuf>(buf: T) -> BufferResult<u8> {
    type IntType = u8;
    let buf = buf.into_buf();
    let nondigits: Vec<char> = buf
        .bytes()
        .iter()
        .filter_map(|x| {
            let c = *x as char;
            if !c.is_digit(10) {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    if nondigits.len() != 0 {
        return Err(UnbufferError::InvalidDecimalDigit(nondigits.into()));
    }
    let zero: IntType = 0;
    buf.bytes().iter().fold(Ok(zero), |acc, x| {
        acc.and_then(|acc_int| {
            if acc_int >= IntType::max_value() / 10 {
                Err(UnbufferError::DecimalOverflow)
            } else {
                let digit_val = (*x as char).to_digit(10).unwrap() as IntType;
                Ok(acc_int * 10 + digit_val)
            }
        })
    })
}
impl Unbuffer for CookieData {
    fn do_unbuffer(buf: &mut Bytes) -> BufferResult<Self> {
        check_expected(buf, MAGIC_PREFIX)?;
        let major: u8 = parse_decimal_u8(buf.split_to(2))?;
        // remove dot
        check_expected(buf, b".")?;
        let minor: u8 = parse_decimal_u8(buf.split_to(2))?;
        // remove spaces
        check_expected(buf, b"  ")?;
        let log_mode: u8 = parse_decimal_u8(buf.split_to(1))?;
        Ok(CookieData {
            version: Version { major, minor },
            log_mode: Some(log_mode),
        })
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
