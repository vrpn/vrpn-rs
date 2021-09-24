// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::cookie::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData},
    VrpnError,
};
use bytes::{Bytes, BytesMut};
use futures::AsyncRead;
use futures::{prelude::*, AsyncReadExt};

pub async fn read_into_bytes_mut<T: AsyncRead + Unpin>(
    stream: &mut T,
    buf: &mut BytesMut,
) -> async_std::io::Result<usize> {
    let mut before = buf.split();
    let n = stream.read(buf).await?;
    unsafe {
        buf.set_len(n);
    }
    before.unsplit(buf.clone());
    *buf = before;
    Ok(n)
}

pub async fn read_n_into_bytes_mut<T: AsyncRead + Unpin>(
    stream: &mut T,
    buf: &mut BytesMut,
    max_len: usize,
) -> async_std::io::Result<usize> {
    buf.reserve(max_len);
    let orig_cap = buf.capacity();

    let mut before = buf.split();
    let after_limit = buf.split_off(max_len);
    let result = match AsyncReadExt::read_exact(stream, buf).await {
        Ok(_) => unsafe {
            buf.set_len(max_len);
            Ok(max_len)
        },
        Err(e) => Err(e),
    };

    before.unsplit(buf.clone());
    before.unsplit(after_limit);
    *buf = before;

    assert_eq!(orig_cap, buf.capacity());
    result
}

/// Reads a cookie's worth of data into a temporary buffer.
pub async fn read_cookie<T>(stream: &mut T, buf: &mut BytesMut) -> Result<(), VrpnError>
where
    T: AsyncRead + Unpin,
{
    // // buf.resize(CookieData::constant_buffer_size(), 0);
    // buf.reserve(CookieData::constant_buffer_size());
    // let orig_cap = buf.capacity();
    // let n = {
    //     let buf = buf.clone();
    // let mut after_cookie = buf.split_off(CookieData::constant_buffer_size());
    // stream.read_exact(buf).await?;
    // // let mut buf = Vec::with_capacity(CookieData::constant_buffer_size());
    // // stream.read(buf).await?;
    // after_cookie.unsplit(buf.clone());
    // }
    // assert_eq!(orig_cap, buf.capacity());
    // Ok(())
    read_n_into_bytes_mut(stream, buf, CookieData::constant_buffer_size()).await?;
    Ok(())
}
