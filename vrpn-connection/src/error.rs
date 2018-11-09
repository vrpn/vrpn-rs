// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use vrpn_base::types::IdType;
use vrpn_buffer::{buffer, unbuffer};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidRemoteId(id: IdType) {
            display("invalid remote id {}", id)
        }
        InvalidLocalId(id: IdType) {
            display("invalid local id {}", id)
        }
        InvalidTypeId(id: IdType){
            display("invalid message type id {}", id)
        }
        EmptyEntry {
            description("empty translation table entry")
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
        TooManyHandlers {
            description("too many handlers")
        }
        HandlerNotFound {
            description("handler not found")
        }
        GenericErrorReturn {
            description("handler returned an error")
        }
        TooManyMappings {
            description("too many mappings")
        }
        NotSystemMessage {
            description("a non-system message was forwarded to Endpoint::handle_system_message()")
        }
        UnrecognizedSystemMessage(id: IdType) {
            display("un-recognized system message id {}", id)
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
        ConsErrors(err: Box<Error>, tail: Box<Error>) {
            cause(err)
            display("{}, {}", err, tail)
        }
    }
}
impl Error {
    pub fn append(self, new_err: Error) -> Error {
        Error::ConsErrors(Box::new(new_err), Box::new(self))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn append_error(old: Result<()>, new_err: Error) -> Result<()> {
    match old {
        Err(old_e) => Err(old_e.append(new_err)),
        Ok(()) => Err(new_err),
    }
}
