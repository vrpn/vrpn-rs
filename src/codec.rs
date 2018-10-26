// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{BufMut, Bytes, BytesMut};
use constants;
use std::{error, fmt, io};
pub use tokio::codec::{Decoder, Encoder};
pub use tokio::io::AsyncRead;
use types::{CookieData, Version};

quick_error!{
#[derive(Debug)]
pub enum CodecError {
    IoError(err: io::Error) {
        from()
        cause(err)
        description(err.description())
    }
    UnexpectedDataAscii(actual: Bytes, expected: String) {
        display("unexpected data: expected '{}', got '{:?}'", expected, &actual[..])
    }
    DecimalParseError {
        description("decimal number parse error")
    }
    GeneralParseError {
        description("parse error")
    }
    VersionMismatch(actual: Version, expected: Version) {
        display(
                "Version mismatch: expected something compatible with {}, got {}",
                expected, actual)
    }
}

}
pub type Result<T> = std::result::Result<T, CodecError>;

/*
#[derive(Debug)]
pub enum CodecError {
    IoError(io::Error),
    //TokioIoError(tokio::io::Error),
    UnexpectedDataAscii(Bytes, String),
    DecimalParseError,
    GeneralParseError,
    VersionMismatch(Version, Version),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CodecError::IoError(e) => write!(f, "{}", e),
            //        CodecError::TokioIoError(e) => write!(f, "{}", e),
            CodecError::UnexpectedDataAscii(actual, expected) => {
                write!(f, "expected '{}', got '{:?}'", expected, &actual[..])
            }
            CodecError::DecimalParseError => write!(f, "decimal number parse error"),
            CodecError::GeneralParseError => write!(f, "parse error"),
            CodecError::VersionMismatch(actual, expected) => write!(
                f,
                "Version mismatch: expected something compatible with {}, got {}",
                expected, actual
            ),
        }
    }
}

impl error::Error for CodecError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            CodecError::IoError(e) => Some(e),
            //CodecError::TokioIoError(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for CodecError {
    fn from(e: io::Error) -> CodecError {
        CodecError::IoError(e)
    }
}
*/
/*
impl From<tokio::io::Error> for CodecError {
    fn from(e: tokio::io::Error) -> CodecError {
        CodecError::TokioIoError(e)
    }
}
*/

#[derive(Debug)]
pub struct MagicCookie;

fn decode_decimal(buf: &mut BytesMut) -> Result<u8> {
    let mut val: u8 = 0;
    while !buf.is_empty() {
        let c = buf[0];
        buf.advance(1);
        if c < ('0' as u8) || c > ('9' as u8) {
            return Err(CodecError::DecimalParseError);
        }
        val *= 10;
        val += c - ('0' as u8);
    }
    Ok(val)
}

fn split_to_expected<'a>(buf: &'a mut BytesMut, expected: &str) -> Result<()> {
    let split = buf.split_to(expected.len());
    if split == expected {
        return Ok(());
    }
    Err(CodecError::UnexpectedDataAscii(
        split.freeze(),
        String::from(expected),
    ))
}
impl Decoder for MagicCookie {
    type Item = CookieData;
    type Error = CodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
        if buf.len() >= constants::COOKIE_SIZE {
            let mut data: CookieData = Default::default();
            split_to_expected(buf, constants::MAGIC_PREFIX)?;
            data.version.major = decode_decimal(&mut buf.split_to(2))?;
            // remove dot
            split_to_expected(buf, ".")?;
            data.version.minor = decode_decimal(&mut buf.split_to(2))?;
            // remove spaces
            split_to_expected(buf, "  ")?;
            data.log_mode = Some(decode_decimal(&mut buf.split_to(1))?);
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}.{:02}", self.major, self.minor)
    }
}

impl fmt::Display for CookieData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}  {}",
            constants::MAGIC_PREFIX,
            self.version,
            self.log_mode.unwrap_or(0)
        )
    }
}

impl Encoder for MagicCookie {
    type Item = CookieData;
    type Error = io::Error;
    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        buf.put(format!("{}", data));
        buf.resize(::constants::COOKIE_SIZE, 0);
        Ok(())
    }
}

pub fn check_ver_nonfile_compatible(ver: Version) -> Result<()> {
    if ver.major == constants::MAGIC_DATA.major {
        Ok(())
    } else {
        Err(CodecError::VersionMismatch(ver, constants::MAGIC_DATA))
    }
}

pub fn check_ver_file_compatible(ver: Version) -> Result<()> {
    if ver.major == constants::FILE_MAGIC_DATA.major {
        Ok(())
    } else {
        Err(CodecError::VersionMismatch(ver, constants::FILE_MAGIC_DATA))
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
        assert!(super::check_ver_nonfile_compatible(super::constants::FILE_MAGIC_DATA).is_ok());
        assert!(super::check_ver_file_compatible(super::constants::FILE_MAGIC_DATA).is_ok());
    }
}
