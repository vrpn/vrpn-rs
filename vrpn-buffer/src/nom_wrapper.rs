// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use nom::{self, Err as NomError, IResult};
use traits::{
    unbuffer::{Error, Output, Result},
    BytesRequired,
};

fn bytes_required_exactly(v: nom::Needed) -> BytesRequired {
    match v {
        nom::Needed::Unknown => BytesRequired::Unknown,
        nom::Needed::Size(n) => BytesRequired::Exactly(n),
    }
}

fn bytes_required_at_least(v: nom::Needed) -> BytesRequired {
    match v {
        nom::Needed::Unknown => BytesRequired::Unknown,
        nom::Needed::Size(n) => BytesRequired::AtLeast(n),
    }
}

fn call_nom_parser_impl<'a, T, F, G>(buf: &'a Bytes, f: F, make_bytes_required: G) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&'a [u8]) -> IResult<&'a [u8], T>,
    G: FnOnce(nom::Needed) -> BytesRequired,
{
    // shallow copy
    let buf_copy = buf.clone();
    match f(buf) {
        Ok((remaining, data)) => Ok(Output::from_slice(buf, remaining, data)),
        Err(NomError::Incomplete(n)) => Err(Error::NeedMoreData(make_bytes_required(n), buf.clone())),
        Err(e) => Err(From::from(e)),
    }
}

pub fn call_nom_parser<'a, T, F>(buf: &'a Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&'a [u8]) -> IResult<&'a [u8], T>,
{
    call_nom_parser_impl(buf, f, bytes_required_at_least)
}

pub fn call_nom_parser_constant_length<'a, T, F>(buf: &'a Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&'a [u8]) -> IResult<&'a [u8], T>,
{
    call_nom_parser_impl(buf, f, bytes_required_exactly)
}

#[cfg(test)]
mod tests {
    use super::call_nom_parser;
    use bytes::Bytes;
    const ab: &[u8] = b"ab";
    const abc: &[u8] = b"abc";
    const def: &[u8] = b"def";
    const abcdef: &[u8] = b"abcdef";
    named!(findabc<&[u8], &[u8]>, tag!(abc));

    #[test]
    fn nom_bytes() {
        let buf = Bytes::from_static(abcdef);
        let (remaining, result) = findabc(&buf).unwrap();
        assert_eq!(remaining, def);
        assert_eq!(result, abc);
        let remaining_bytes = buf.slice_ref(remaining);
        assert_eq!(remaining_bytes, def);
    }

    #[test]
    fn call_parser() {
        let buf = Bytes::from_static(abcdef);
        let output = call_nom_parser(&buf, findabc).unwrap();
        assert_eq!(output.remaining(), def);
        assert_eq!(output.borrow_data(), &abc);
    }
}
