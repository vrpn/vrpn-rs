// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::cookie::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData},
    VrpnError,
};
use bytes::{Bytes, BytesMut};
use futures::{AsyncRead, AsyncReadExt};

/// Reads a cookie's worth of data into a temporary buffer.
pub async fn read_cookie<T>(stream: &mut T) -> Result<Vec<u8>, VrpnError>
where
    T: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(CookieData::constant_buffer_size());
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}
