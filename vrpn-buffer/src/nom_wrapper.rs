// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use nom::{self, Err as NomError, IResult};
use size::{BytesRequired, ConstantBufferSize};
use unbuffer::{Error, Output, Result};

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

fn call_nom_parser_impl<T, F, G>(buf: Bytes, f: F, make_bytes_required: G) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&[u8]) -> IResult<&[u8], T>,
    G: FnOnce(nom::Needed) -> BytesRequired,
{
    // shallow copy
    let buf_copy = buf.clone();
    match f(&buf_copy) {
        Ok((remaining, data)) => Ok(Output::from_slice(buf, remaining, data)),
        Err(NomError::Incomplete(n)) => Err(Error::NeedMoreData(make_bytes_required(n), buf)),
        Err(e) => Err(From::from(e)),
    }
}

pub fn call_nom_parser<T, F>(buf: Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&[u8]) -> IResult<&[u8], T>,
{
    call_nom_parser_impl(buf, f, bytes_required_at_least)
}

pub fn call_nom_parser_constant_length<T, F>(buf: Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&[u8]) -> IResult<&[u8], T>,
{
    call_nom_parser_impl(buf, f, bytes_required_exactly)
}
