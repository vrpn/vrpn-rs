// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use buffer::Buffer;
use bytes::{Buf, BufMut, Bytes, IntoBuf};
use nom_wrapper::call_nom_parser_constant_length;
use size::ConstantBufferSize;
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::result;
use unbuffer::{AndThenMap, Error, Output, Result, Unbuffer};
use vrpn_base::constants::{self, COOKIE_SIZE, MAGIC_PREFIX};
use vrpn_base::cookie::{CookieData, Version};

const COOKIE_PADDING: &[u8] = b"\0\0\0\0\0";

impl ConstantBufferSize for CookieData {
    fn constant_buffer_size() -> usize {
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

fn from_dec<'a>(input: &'a [u8]) -> result::Result<u8, ParseIntError> {
    u8::from_str_radix(&String::from_utf8_lossy(input), 10)
}

named!(dec_digits_1<&[u8], u8>, map_res!(take!(1), from_dec));
named!(dec_digits_2<&[u8], u8>, map_res!(take!(2), from_dec));

named!(cookie<&[u8], CookieData>,
    do_parse!(
        tag!(MAGIC_PREFIX) >>
        major: dec_digits_2 >>
        tag!(".") >>
        minor: dec_digits_2 >>
        tag!("  ") >>
        mode: dec_digits_1 >>
        tag!(COOKIE_PADDING) >>
        (CookieData{version: Version{major, minor}, log_mode: Some(mode)})
        ));
#[cfg(test)]
mod tests {
    #[test]
    fn basics() {
        use super::*;
        assert_eq!(from_dec(b"1"), Ok(1_u8));
        assert_eq!(from_dec(b"12"), Ok(12_u8));
        assert_eq!(dec_digits_1(b"1"), Ok((&b""[..], 1_u8)));
        assert_eq!(dec_digits_2(b"12"), Ok((&b""[..], 12_u8)));
    }

}

impl Unbuffer for CookieData {
    fn unbuffer(buf: Bytes) -> Result<Output<Self>> {
        call_nom_parser_constant_length(buf, cookie)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_decimal() {
        /*
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
        */
    }
}
