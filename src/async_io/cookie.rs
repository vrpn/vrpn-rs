// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    constants::{FILE_MAGIC_DATA, MAGIC_DATA},
    cookie::{check_ver_file_compatible, check_ver_nonfile_compatible},
    prelude::BytesMutExtras,
    ConstantBufferSize, CookieData, Error, Unbuffer,
};
use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Writes the supplied cookie to a stream.
async fn write_cookie<T>(stream: &mut T, cookie: CookieData) -> Result<(), Error>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    let buf = BytesMut::new().allocate_and_buffer(cookie)?.freeze();
    stream.write_all(&buf).await?;
    Ok(())
}

/// Reads a cookie's worth of data into a temporary buffer.
///
/// Future resolves to (stream, buffer) on success.
async fn read_cookie<T>(stream: &mut T) -> Result<Vec<u8>, Error>
where
    T: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(CookieData::constant_buffer_size());
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Writes the "non-file" magic cookie to the stream.
///
/// Future resolves to the provided stream on success.
pub(crate) async fn send_nonfile_cookie<T>(stream: &mut T) -> Result<(), Error>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    write_cookie(stream, CookieData::from(MAGIC_DATA)).await
}

/// Writes the "file" magic cookie to the stream.
///
/// Future resolves to the provided stream on success.
pub(crate) async fn send_file_cookie<T>(stream: &mut T) -> Result<(), Error>
where
    T: tokio::io::AsyncWrite + Unpin,
{
    write_cookie(stream, CookieData::from(FILE_MAGIC_DATA)).await
}

/// Reads a cookie's worth of data from the stream, and cheacks to make sure it is the right version.
pub(crate) async fn read_and_check_nonfile_cookie<T>(stream: &mut T) -> Result<(), Error>
where
    T: tokio::io::AsyncRead + Unpin,
{
    let read_buf: Vec<u8> = read_cookie(stream).await?;
    let mut buf = Bytes::from(read_buf);
    let msg = CookieData::unbuffer_ref(&mut buf)?;
    check_ver_nonfile_compatible(msg.version)?;
    Ok(())
}

/// Reads a cookie's worth of data from the stream, and cheacks to make sure it is the right version.
///
/// Future resolves to the provided stream on success.
pub(crate) async fn read_and_check_file_cookie<T>(stream: &mut T) -> Result<(), Error>
where
    T: tokio::io::AsyncRead + Unpin,
{
    let read_buf: Vec<u8> = read_cookie(stream).await?;
    let mut buf = Bytes::from(read_buf);
    let msg = CookieData::unbuffer_ref(&mut buf)?;
    check_ver_file_compatible(msg.version)?;
    Ok(())
}
