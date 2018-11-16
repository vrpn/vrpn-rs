// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// A simple, synchronous-IO client for testing purposes.

extern crate bytes;
extern crate vrpn;

use bytes::{Bytes, BytesMut};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
};
use vrpn::{
    async_io::codec::peek_u32, constants::MAGIC_DATA, cookie::check_ver_nonfile_compatible,
    message::MessageSize, prelude::*, ConstantBufferSize, CookieData, Message, Result,
    SequencedGenericMessage, Unbuffer,
};

fn write_cookie<T>(stream: &mut T, cookie: CookieData) -> Result<()>
where
    T: Write,
{
    BytesMut::new().allocate_and_buffer(cookie).and_then(|buf| {
        stream.write_all(&buf.freeze())?;
        Ok(())
    })
}
fn read_cookie<T>(stream: &mut T) -> Result<Vec<u8>>
where
    T: Read,
{
    let mut buf = vec![0u8; CookieData::constant_buffer_size()];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}
fn main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();
    let mut stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    write_cookie(&mut stream, CookieData::from(MAGIC_DATA))?;
    let cookie_buf = read_cookie(&mut stream)?;
    let mut cookie_buf = Bytes::from(&cookie_buf[..]);

    CookieData::unbuffer_ref(&mut cookie_buf)
        .and_then(|msg| check_ver_nonfile_compatible(msg.version))?;

    // Not actually doing anything with the messages here, just receiving them and printing them.
    loop {
        let mut buf = BytesMut::new();
        buf.resize(24, 0);
        stream.read_exact(buf.as_mut())?;

        let total_len = peek_u32(&buf.clone().freeze())?.unwrap();
        let size = MessageSize::from_length_field(total_len);

        let mut body_buf = BytesMut::new();
        body_buf.resize(size.padded_body_size(), 0);
        stream.read_exact(body_buf.as_mut())?;
        buf.extend_from_slice(&body_buf[..]);
        let mut buf = buf.freeze();

        let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut buf)?;
        eprintln!("{:?}", Message::from(unbuffered));
    }
}
