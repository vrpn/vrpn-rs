// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

// A simple, synchronous-IO client for testing purposes.
// Doesn't use any of the async-io stuff in the vrpn crate,
// so this is durable even if Tokio totally changes everything.
//
// Admittedly, it also doesn't use any of the message type registry or dispatch abilities.

extern crate bytes;
extern crate vrpn;

use bytes::{Bytes, BytesMut};
use std::{
    io::Read,
    net::{SocketAddr, TcpStream},
};
use vrpn::{
    buffer_unbuffer::{peek_u32, Unbuffer},
    data_types::{
        cookie::check_ver_nonfile_compatible, CookieData, Message, MessageSize,
        SequencedGenericMessage,
    },
    sync_io::{read_cookie, write_cookie},
    Result,
};

fn main() -> Result<()> {
    let addr: SocketAddr = "127.0.0.1:3883".parse().unwrap();
    let mut stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;

    // We first write our cookie, then read and check the server's cookie, before the loop.
    write_cookie(&mut stream, CookieData::make_cookie())?;
    let cookie_buf = read_cookie(&mut stream)?;
    let mut cookie_buf = Bytes::copy_from_slice(&cookie_buf[..]);

    let msg = CookieData::unbuffer_ref(&mut cookie_buf)?;
    check_ver_nonfile_compatible(msg.version)?;

    // Not actually doing anything with the messages here, just receiving them and printing them.
    loop {
        let mut buf = BytesMut::new();

        // Read the message header and padding
        buf.resize(24, 0);
        stream.read_exact(buf.as_mut())?;

        // Peek the size field, to compute the MessageSize.
        let total_len = peek_u32(&buf.clone().freeze()).unwrap();
        let size = MessageSize::from_length_field(total_len);

        // Read the body of the message
        let mut body_buf = BytesMut::new();
        body_buf.resize(size.padded_body_size(), 0);
        stream.read_exact(body_buf.as_mut())?;

        // Combine the body with the header
        buf.extend_from_slice(&body_buf[..]);
        let mut buf = buf.freeze();

        // Unbuffer the message.
        let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut buf)?;
        eprintln!("{:?}", Message::from(unbuffered));
    }
}
