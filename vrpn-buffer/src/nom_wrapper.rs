// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use nom::{self, Err as NomError, IResult};
use std::borrow::Borrow;
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

fn update_buf_for_consumed_bytes(buf: &mut Bytes, remaining: &[u8]) {
    let consumed = buf.len() - remaining.len();
    buf.advance(consumed);
}

fn update_buf_from_remaining_bytes(buf: &mut Bytes, subset: Bytes) {
    let bytes_p = buf.as_ptr() as usize;
    let bytes_len = buf.len();

    let sub_p = subset.as_ptr() as usize;
    let sub_len = subset.len();
    assert!(sub_p >= bytes_p, "subset begins before the buffer");
    assert!(
        sub_p + sub_len == bytes_p + bytes_len,
        "subset doesn't end at the same point as the buffer"
    );
    let consumed = bytes_len - sub_len;
    buf.advance(consumed);
}
/*
struct NomParser<'a, T> {
    buf: &'a mut Bytes,
    phantom: std::marker::PhantomData<T>,
}
impl<'a, T> NomParser<'a, T> {
    fn call<F, G>(buf: &'a mut Bytes, f: F, make_bytes_required: G) -> Result<Output<T>>
    where
        F: FnOnce(&[u8]) -> IResult<&[u8], T>,
        G: FnOnce(nom::Needed) -> BytesRequired,
    {
        let (Output(data), consumed) = NomParser {
            buf,
            phantom: Default::default(),
        }
        .do_call(f, make_bytes_required)?;

        buf.advance(consumed);
        Ok(Output(data))
    }
    fn do_call<F, G>(self, f: F, make_bytes_required: G) -> Result<(Output<T>, usize)>
    where
        F: FnOnce(&[u8]) -> IResult<&[u8], T>,
        G: FnOnce(nom::Needed) -> BytesRequired,
    {
        match f(self.buf) {
            Ok((remaining, data)) => Ok((Output(data), self.buf.len() - remaining.len())),
            Err(NomError::Incomplete(n)) => Err(Error::NeedMoreData(make_bytes_required(n))),
            Err(e) => Err(Error::ParseError(e.to_string())),
        }
    }
}
*/
fn call_nom_parser_impl<'a, T, F, G>(buf: &mut Bytes, f: F, make_bytes_required: G) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&'a [u8]) -> IResult<&'a [u8], T>,
    G: FnOnce(nom::Needed) -> BytesRequired,
{
    let my_buf = buf.clone();
    match f(buf.borrow()) {
        Ok((remaining, data)) => {
            let consumed = buf.len() - remaining.len();
            buf.advance(consumed);

            Ok(Output(data))
        }
        Err(NomError::Incomplete(n)) => Err(Error::NeedMoreData(make_bytes_required(n))),
        Err(e) => Err(Error::ParseError(e.to_string())),
    }
}
trait NomParser<T> {
    fn parse<'a>(buf: &'a [u8]) -> IResult<&'a [u8], T>;
    fn call(buf: &mut Bytes) -> Result<Output<T>> {
        let my_buf = buf.clone();
        match Self::parse(&my_buf) {
            Ok((remaining, data)) => {
                let consumed = buf.len() - remaining.len();
                buf.advance(consumed);

                Ok(Output(data))
            }
            Err(NomError::Incomplete(n)) => Err(Error::NeedMoreData(bytes_required_at_least(n))),
            Err(e) => Err(Error::ParseError(e.to_string())),
        }
    }
}
pub fn call_nom_parser<'r, T, F>(buf: &mut Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    F: FnOnce(&'r [u8]) -> IResult<&'r [u8], T>,
{
    
    call_nom_parser_impl(buf, f, bytes_required_at_least)
    //NomParser::call(buf, f, bytes_required_at_least)
    
    //Err(Error::ParseError("err".to_string()))
}

pub fn call_nom_parser_constant_length<'b, T, F>(buf: &'b mut Bytes, f: F) -> Result<Output<T>>
where
    T: Sized,
    for<'a> F: FnOnce(&'a [u8]) -> IResult<&'a [u8], T>,
{
    call_nom_parser_impl(buf, f, bytes_required_exactly)
}

#[cfg(test)]
mod tests {
    use super::{call_nom_parser, Error, IResult, NomParser, Output, Result};
    use bytes::Bytes;
    use bytes::IntoBuf;
    const ab: &[u8] = b"ab";
    const abc: &[u8] = b"abc";
    const def: &[u8] = b"def";
    const abcdef: &[u8] = b"abcdef";
    named!(findabc<&[u8], &[u8]>, tag!(abc));

    struct AbcParser;
    impl<'b> NomParser<&'b [u8]> for AbcParser {
        fn parse<'a>(buf: &'a [u8]) -> IResult<&'a [u8], &'b [u8]> {
            findabc(buf)
        }
    }
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
        let mut buf = Bytes::from_static(abcdef);
        let output = call_nom_parser(&mut buf, findabc).unwrap();
        //let output = AbcParser::call(&mut buf).unwrap(); //call_nom_parser(&mut buf, findabc).unwrap();
        assert_eq!(buf, def);
        assert_eq!(output.borrow_data(), &abc);
    }

}
