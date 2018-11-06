// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    base::{
        cookie::{self, check_ver_nonfile_compatible, CookieData},
        types::{LocalId, RemoteId, SenderId},
    },
    buffer::{buffer, unbuffer, Buffer, BufferSize, ConstantBufferSize, Output, Unbuffer},
    connection::translationtable::TranslationTable,
    prelude::*,
    *,
};
use bytes::{Bytes, BytesMut};
use tokio::{io, net::TcpStream, prelude::*};

quick_error! {
    #[derive(Debug)]
    pub enum ConnectError {
        VersionError(err: cookie::VersionError) {
            from()
            display("version error: {}", err)
            cause(err)
        }
        UnbufferError(err: unbuffer::Error) {
            from()
            display("unbuffer error: {}", err)
            cause(err)
        }
        BufferError(err: buffer::Error) {
            from()
            display("buffer error: {}", err)
            cause(err)
        }
        IoError(err: io::Error) {
            from()
            display("IO error: {}", err)
            cause(err)
        }
    }
}
