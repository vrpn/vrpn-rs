// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{BytesMutExtras, ConstantBufferSize, UnbufferFrom},
    data_types::{
        constants::COOKIE_SIZE,
        cookie::{check_ver_file_compatible, check_ver_nonfile_compatible, CookieData},
    },
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
    let mut buf = [0u8; COOKIE_SIZE];
    stream.read_exact(&mut buf).await?;
    Ok(buf.to_vec())
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

#[cfg(test)]
mod tests {
    use crate::{
        buffer_unbuffer::{BytesMutExtras, ConstantBufferSize},
        data_types::{constants::COOKIE_SIZE, CookieData},
    };
    use async_std::task;
    use bytes::{Bytes, BytesMut};
    use futures::io::Cursor;

    fn get_cookie_buf(file_cookie: bool) -> Bytes {
        assert_eq!(CookieData::constant_buffer_size(), COOKIE_SIZE);
        BytesMut::allocate_and_buffer(if file_cookie {
            CookieData::make_file_cookie()
        } else {
            CookieData::make_cookie()
        })
        .expect("should buffer cookie")
        .freeze()
    }

    #[test]
    fn read_cookie() {
        {
            let cookie = get_cookie_buf(false);
            let mut reader = Cursor::new(&cookie[..]);
            let read_buf = task::block_on(super::read_cookie(&mut reader)).unwrap();
            assert_eq!(CookieData::constant_buffer_size(), read_buf.len());
            assert_eq!(&cookie[..], &read_buf[..]);
        }
        {
            let cookie = get_cookie_buf(true);
            let mut reader = Cursor::new(&cookie[..]);
            let read_buf = task::block_on(super::read_cookie(&mut reader)).unwrap();
            assert_eq!(CookieData::constant_buffer_size(), read_buf.len());
            assert_eq!(&cookie[..], &read_buf[..]);
        }
    }

    #[test]
    fn check_cookie() {
        {
            let cookie = get_cookie_buf(false);
            let mut reader = Cursor::new(&cookie[..]);
            task::block_on(super::read_and_check_nonfile_cookie(&mut reader))
                .expect("checking cookie should pass");
        }
        {
            let cookie = get_cookie_buf(true);
            let mut reader = Cursor::new(&cookie[..]);
            task::block_on(super::read_and_check_file_cookie(&mut reader))
                .expect("checking cookie should pass");
        }
    }

    #[test]
    fn write_cookie() {
        {
            let mut writer = Cursor::new(vec![0u8; COOKIE_SIZE]);
            task::block_on(super::send_nonfile_cookie(&mut writer)).unwrap();
            let write_buf = writer.into_inner();
            assert_eq!(&get_cookie_buf(false), &write_buf);
        }
        {
            let mut writer = Cursor::new(vec![0u8; COOKIE_SIZE]);
            task::block_on(super::send_file_cookie(&mut writer)).unwrap();
            let write_buf = writer.into_inner();
            assert_eq!(&get_cookie_buf(true), &write_buf);
        }
    }
}
