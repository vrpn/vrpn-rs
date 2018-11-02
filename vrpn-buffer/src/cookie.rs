// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
// use nom_wrapper::call_nom_parser_constant_length;
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::result;
use traits::{
    buffer::{self, Buffer},
    unbuffer::{self, check_expected, Output, OutputResultExtras, Unbuffer},
    ConstantBufferSize,
};
use vrpn_base::constants::{self, COOKIE_SIZE, MAGIC_PREFIX};
use vrpn_base::cookie::{CookieData, Version};

const COOKIE_PADDING: &[u8] = b"\0\0\0\0\0";

impl ConstantBufferSize for CookieData {
    fn constant_buffer_size() -> usize {
        COOKIE_SIZE
    }
}

impl Buffer for CookieData {
    fn buffer<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        if buf.remaining_mut() < Self::constant_buffer_size() {
            return Err(buffer::Error::OutOfBuffer);
        }
        buf.put(self.to_string());
        buf.put(COOKIE_PADDING);
        Ok(())
    }
}
#[inline]
fn from_dec(input: &[u8]) -> result::Result<u8, ParseIntError> {
    u8::from_str_radix(&String::from_utf8_lossy(input), 10)
}

#[inline]
fn dec_digits(buf: &mut Bytes, n: usize) -> result::Result<u8, ParseIntError> {
    from_dec(&buf.split_to(n))
}
// named!(dec_digits_1<&[u8], u8>, map_res!(take!(1), from_dec));
// named!(dec_digits_2<&[u8], u8>, map_res!(take!(2), from_dec));

// named!(
//     cookie<&[u8], CookieData>,
//     do_parse!(
//         tag!(MAGIC_PREFIX)
//             >> major: dec_digits_2
//             >> tag!(".")
//             >> minor: dec_digits_2
//             >> tag!("  ")
//             >> mode: dec_digits_1
//             >> tag!(COOKIE_PADDING)
//             >> (CookieData {
//                 version: Version { major, minor },
//                 log_mode: Some(mode)
//             })
//     )
// );

impl Unbuffer for CookieData {
    fn unbuffer(buf: &mut Bytes) -> unbuffer::Result<Output<Self>> {
        // call_nom_parser_constant_length(buf, cookie)

        check_expected(buf, MAGIC_PREFIX)?;
        let major: u8 = dec_digits(buf, 2)?;
        // remove dot
        check_expected(buf, b".")?;
        let minor: u8 = dec_digits(buf, 2)?;
        // remove spaces
        check_expected(buf, b"  ")?;
        let log_mode: u8 = dec_digits(buf, 1)?;
        Ok(Output(CookieData {
            version: Version { major, minor },
            log_mode: Some(log_mode),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn magic_size() {
        // Make sure the size is right.
        use super::{constants, Buffer, CookieData};

        let mut magic_cookie = CookieData::from(constants::MAGIC_DATA);
        magic_cookie.log_mode = Some(0);
        assert_eq!(magic_cookie.required_buffer_size(), constants::COOKIE_SIZE);

        let mut buf = Vec::new();
        magic_cookie
            .buffer(&mut buf)
            .expect("Buffering needs to succeed");
        assert_eq!(buf.len(), constants::COOKIE_SIZE);
    }

    #[test]
    fn roundtrip() {
        use super::{constants, Buffer, CookieData, Unbuffer};
        use bytes::BytesMut;

        let mut magic_cookie = CookieData::from(constants::MAGIC_DATA);
        magic_cookie.log_mode = Some(0);
        let mut buf = BytesMut::with_capacity(magic_cookie.required_buffer_size());
        magic_cookie
            .buffer(&mut buf)
            .expect("Buffering needs to succeed");
        let mut buf = buf.freeze();
        assert_eq!(CookieData::unbuffer(&mut buf).unwrap().data(), magic_cookie);
    }

    #[test]
    fn basics() {
        assert_eq!(from_dec(b"1"), Ok(1_u8));
        assert_eq!(from_dec(b"12"), Ok(12_u8));
    }
    #[test]
    fn dec_digits_fn() {
        {
            let mut buf = Bytes::from_static(b"1");
            assert_eq!(dec_digits(&mut buf, 1), Ok(1_u8));
            assert_eq!(buf.len(), 0);
        }
        {
            let mut buf = Bytes::from_static(b"12");
            assert_eq!(dec_digits(&mut buf, 2), Ok(12_u8));
            assert_eq!(buf.len(), 0);
        }
    }
    #[test]
    fn parse_decimal() {
        fn parse_decimal_u8(v: &'static [u8]) -> u8 {
            super::from_dec(v).unwrap()
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
