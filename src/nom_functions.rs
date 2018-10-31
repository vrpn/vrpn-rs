// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use constants::MAGIC_PREFIX;
use cookie::{CookieData, Version};

fn from_dec<'a>(input: &'a [u8]) -> Result<u8, std::num::ParseIntError> {
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
        tag!("\0\0\0\0\0") >>
        (CookieData{version: Version{major, minor}, log_mode: Some(mode)})
        )); //

pub fn parse_cookie<'a>(s: &'a [u8]) -> Result<(&'a [u8], CookieData), nom::Err<&'a [u8]>> {
    match cookie(s) {
        Ok((remaining, data)) => Ok((remaining, data)),
        Err(e) => Err(e),
    }
}
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
