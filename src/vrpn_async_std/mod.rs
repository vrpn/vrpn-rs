// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::cookie::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData},
    VrpnError,
};
use bytes::{Bytes, BytesMut};
use futures::prelude::*;
use futures::AsyncRead;

/// Reads a cookie's worth of data into a temporary buffer.
pub async fn read_cookie<T>(stream: &mut T, buf: &mut BytesMut) -> Result<Bytes, VrpnError>
where
    T: AsyncRead + Unpin,
{
    buf.resize(CookieData::constant_buffer_size(), 0);
    let mut cookie_buf = buf.split();
    stream.read_exact(&mut cookie_buf).await?;
    // let mut buf = Vec::with_capacity(CookieData::constant_buffer_size());
    // stream.read(buf).await?;
    Ok(cookie_buf.freeze())
}
