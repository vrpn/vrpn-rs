// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants::{FILE_MAGIC_DATA, MAGIC_DATA},
    cookie::{check_ver_file_compatible, check_ver_nonfile_compatible},
    prelude::{BytesMutExtras, WrappedConstantSize},
    ConstantBufferSize, CookieData, Error, Unbuffer,
};
use bytes::{Bytes, BytesMut};
use futures::Future;
use tokio::prelude::*;
use tokio::io::{self, AsyncReadExt};

/// Writes the supplied cookie to a stream.
///
/// Future resolves to the provided stream on success.
async fn write_cookie<T>(stream: T, cookie: CookieData) -> Result<Item = T, Error = Error>
where
    T: AsyncWrite,
{
    let buf = BytesMut::new().allocate_and_buffer(cookie)?;
    stream
        .write_all(stream, buf.freeze())
        .map(|(stream, _)| stream)
        .await
}

/// Reads a cookie's worth of data into a temporary buffer.
///
/// Future resolves to (stream, buffer) on success.
async fn read_cookie<T>(stream: T) -> Result<Item = (T, Vec<u8>), Error = Error>
where
    T: AsyncRead,
{
    stream.read_exact(vec![0u8; CookieData::constant_buffer_size()]).from_err()
}

fn verify_version_nonfile(msg: CookieData) -> impl Future<Item = (), Error = Error> {
    check_ver_nonfile_compatible(msg.version).into_future()
}

fn verify_version_file(msg: CookieData) -> impl Future<Item = (), Error = Error> {
    check_ver_file_compatible(msg.version).into_future()
}

// /// Writes the "non-file" magic cookie to the stream.
///
/// Future resolves to the provided stream on success.
pub(crate) fn send_nonfile_cookie<T>(stream: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncWrite,
{
    write_cookie(stream, CookieData::from(MAGIC_DATA))
}

/// Writes the "file" magic cookie to the stream.
///
/// Future resolves to the provided stream on success.
pub(crate) fn send_file_cookie<T>(stream: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncWrite,
{
    write_cookie(stream, CookieData::from(FILE_MAGIC_DATA))
}

/// Reads a cookie's worth of data from the stream, and cheacks to make sure it is the right version.
///
/// Future resolves to the provided stream on success.
pub(crate) fn read_and_check_nonfile_cookie<T>(stream: T) -> impl Future<Item = T, Error = Error>
where
    T: AsyncRead,
{
    read_cookie(stream).and_then(|(stream, read_buf)| {
        let mut buf = Bytes::from(&read_buf[..]);
        CookieData::unbuffer_ref(&mut buf)
            .into_future()
            .and_then(verify_version_nonfile)
            .and_then(|()| Ok(stream))
    })
}

/// Reads a cookie's worth of data from the stream, and cheacks to make sure it is the right version.
///
/// Future resolves to the provided stream on success.
pub(crate) async fn read_and_check_file_cookie<T>(stream: T) -> Result<Item = T, Error = Error>
where
    T: AsyncRead,
{
    read_cookie(stream).and_then(|(stream, read_buf)| {
        let mut buf = Bytes::from(&read_buf[..]);
        CookieData::unbuffer_ref(&mut buf)
            .into_future()
            .and_then(verify_version_file)
            .and_then(|()| Ok(stream))
    })
}
