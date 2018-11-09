// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    base::{cookie, GenericMessage},
    buffer::{buffer, unbuffer},
    codec::FramedMessageCodec,
    connection::Error as ConnectionError,
    prelude::*,
};
use futures::sync::mpsc;
use std::io;

quick_error!{
    #[derive(Debug)]
    pub enum Error {
        IoError(err: io::Error) {
            from()
            cause(err)
            display("IO error: {}", err)
        }
        BufferError(err: buffer::Error) {
            from()
            cause(err)
            display("buffer error: {}", err)
        }
        UnbufferError(err: unbuffer::Error) {
            from()
            cause(err)
            display("unbuffer error: {}", err)
        }
        MpscSendError(err: mpsc::SendError<GenericMessage>) {
            from()
            cause(err)
            display("mpsc send error: {}", err)
        }
        WrappedConnectionError(err: ConnectionError) {
            from()
            display("{}", err)
            cause(err)
        }
        VersionError(err: cookie::VersionError) {
            from()
            display("version error: {}", err)
            cause(err)
        }
        Other(err: Box<std::error::Error>) {
            cause(&**err)
            from()
            display("{}", err)
            cause(err)
        }
        OtherMessage(s: String) {
            from()
            display("{}", s)
        }
    }
}
