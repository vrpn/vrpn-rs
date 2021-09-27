// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::cookie::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData},
    VrpnError,
};
use async_std::prelude::*;
use bytes::{Bytes, BytesMut};
use futures::{AsyncRead, AsyncWrite};

/// Writes the supplied cookie to a stream.
async fn write_cookie<T>(stream: &mut T, cookie: CookieData) -> Result<(), VrpnError>
where
    T: AsyncWrite + Unpin,
{
    let buf = BytesMut::allocate_and_buffer(cookie)?.freeze();
    stream.write_all(&buf).await?;
    Ok(())
}

/// Reads a cookie's worth of data into a temporary buffer.
///
/// Future resolves to (stream, buffer) on success.
async fn read_cookie<T>(stream: &mut T) -> Result<Vec<u8>, VrpnError>
where
    T: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(CookieData::constant_buffer_size());
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Writes the "non-file" magic cookie to the stream.
pub async fn send_nonfile_cookie<T>(stream: &mut T) -> Result<(), VrpnError>
where
    T: AsyncWrite + Unpin,
{
    write_cookie(stream, CookieData::make_cookie()).await
}

/// Writes the "file" magic cookie to the stream.
pub async fn send_file_cookie<T>(stream: &mut T) -> Result<(), VrpnError>
where
    T: AsyncWrite + Unpin,
{
    write_cookie(stream, CookieData::make_file_cookie()).await
}

/// Reads a cookie's worth of data from the stream, and checks to make sure it is the right version.
pub async fn read_and_check_nonfile_cookie<T>(stream: &mut T) -> Result<(), VrpnError>
where
    T: AsyncRead + Unpin,
{
    let read_buf: Vec<u8> = read_cookie(stream).await?;
    let mut buf = Bytes::from(read_buf);
    let msg = CookieData::unbuffer_from(&mut buf)?;
    check_ver_nonfile_compatible(msg.version)?;
    Ok(())
}

/// Reads a cookie's worth of data from the stream, and cheacks to make sure it is the right version.
pub async fn read_and_check_file_cookie<T>(stream: &mut T) -> Result<(), VrpnError>
where
    T: AsyncRead + Unpin,
{
    let read_buf: Vec<u8> = read_cookie(stream).await?;
    let mut buf = Bytes::from(read_buf);
    let msg = CookieData::unbuffer_from(&mut buf)?;
    check_ver_file_compatible(msg.version)?;
    Ok(())
}
