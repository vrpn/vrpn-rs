// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::BytesMut;
use futures::{AsyncRead, AsyncReadExt};
pub struct BytesMutReader(BytesMut);

impl BytesMutReader {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(BytesMut::with_capacity(capacity))
    }
    pub async fn read_from<T: AsyncRead + Unpin>(
        self,
        stream: &mut T,
    ) -> async_std::io::Result<Self> {
        let mut buf = self.0;
        let orig_cap = buf.capacity();
        let orig_len = buf.len();
        let mut existing_bytes = buf.split();
        let n = stream.read(&mut buf).await?;
        unsafe {
            buf.set_len(n);
        }
        existing_bytes.unsplit(buf);
        assert_eq!(orig_cap, existing_bytes.capacity());
        assert_eq!(orig_len + n, existing_bytes.len());
        Ok(Self(existing_bytes))
    }
    pub fn clear(&mut self) {
        self.0.clear()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn take_contents(&mut self) -> BytesMut {
        self.0.split()
    }
    pub fn give_back_contents(self, contents: BytesMut) -> Self {
        let mut contents = contents;
        contents.unsplit(self.0);
        Self(contents)
    }
}

pub async fn read_into_bytes_mut<T: AsyncRead + Unpin>(
    stream: &mut T,
    buf: &mut BytesMut,
) -> async_std::io::Result<usize> {
    let orig_cap = buf.capacity();
    let orig_len = buf.len();
    let mut before = buf.split();
    let n = stream.read(buf).await?;
    unsafe {
        buf.set_len(n);
    }
    before.unsplit(buf.clone());
    *buf = before;
    assert_eq!(orig_cap, buf.capacity());
    assert_eq!(orig_len + n, buf.len());
    Ok(n)
}

pub async fn read_n_into_bytes_mut<T: AsyncRead + Unpin>(
    stream: &mut T,
    buf: &mut BytesMut,
    max_len: usize,
) -> async_std::io::Result<usize> {
    buf.reserve(max_len);
    let orig_cap = buf.capacity();
    let orig_len = buf.len();
    let mut local_buf: Vec<u8> = Vec::with_capacity(max_len);
    local_buf.resize(max_len, 0u8);
    async_std::io::ReadExt::read_exact(stream, &mut local_buf).await?;
    buf.extend_from_slice(&local_buf);
    assert_eq!(orig_cap, buf.capacity());
    assert_eq!(orig_len + max_len, buf.len());
    Ok(max_len)
}
