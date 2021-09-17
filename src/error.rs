// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{IdType, Version};
use bytes::Bytes;
use std::{
    fmt::{self, Display},
    ops::Add,
};

/// Expresses how many more bytes we require/expect when parsing a message.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BytesRequired {
    Exactly(usize),
    AtLeast(usize),
    Unknown,
}

impl BytesRequired {
    /// Compares a byte requirement to available bytes in a buffer.
    ///
    /// Note that in this case, Exactly(c) is satisfied by c or anything larger.
    pub fn satisfied_by(&self, buf_size: usize) -> Option<bool> {
        match *self {
            BytesRequired::Exactly(c) => Some(c <= buf_size),
            BytesRequired::AtLeast(c) => Some(c <= buf_size),
            BytesRequired::Unknown => None,
        }
    }
}

impl Add for BytesRequired {
    type Output = BytesRequired;
    fn add(self, other: BytesRequired) -> Self::Output {
        use self::BytesRequired::*;
        match (self, other) {
            (Exactly(a), Exactly(b)) => Exactly(a + b),
            (AtLeast(a), Exactly(b)) => AtLeast(a + b),
            (Exactly(a), AtLeast(b)) => AtLeast(a + b),
            (AtLeast(a), AtLeast(b)) => AtLeast(a + b),
            // Anything else has Unknown as one term.
            _ => Unknown,
        }
    }
}

impl Display for BytesRequired {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BytesRequired::Exactly(n) => write!(f, "exactly {}", n),
            BytesRequired::AtLeast(n) => write!(f, "at least {}", n),
            BytesRequired::Unknown => write!(f, "unknown"),
        }
    }
}

quick_error! {
    /// Error type for the main VRPN crate
    #[derive(Debug)]
    pub enum Error {
        InvalidId(id: IdType) {
            display("invalid id {}", id)
        }
        EmptyEntry {
            display("empty translation table entry")
        }
        OutOfBuffer {
            display("buffering ran out of buffer space")
        }
        NeedMoreData(needed: BytesRequired) {
            display("unbuffering ran out of buffered bytes: need {} additional bytes", needed)
        }
        // InvalidDecimalDigit(chars: Vec<char>) {
        //     display(self_) -> ("got the following non-decimal-digit(s) {}", itertools::join(chars.iter().map(|x : &char| x.to_string()), ","))
        // }
        UnexpectedAsciiData(actual: Bytes, expected: Bytes) {
            display("unexpected data: expected '{:?}', got '{:?}'", &expected[..], &actual[..])
        }
        TooManyHandlers {
            display("too many handlers")
        }
        TooManyMappings {
            display("too many mappings")
        }
        HandlerNotFound {
            display("handler not found")
        }
        GenericErrorReturn {
            display("handler returned an error")
        }
        NotSystemMessage {
            display("a non-system message was forwarded to Endpoint::handle_message_as_system()")
        }
        UnrecognizedSystemMessage(id: IdType) {
            display("un-recognized system message id {}", id)
        }

        VersionMismatch(actual: Version, expected: Version) {
            display(
                    "version mismatch: expected something compatible with {}, got {}",
                    expected, actual)
        }
        Other(err: Box<dyn std::error::Error + Send>) {
            source(&**err)
            display("{}", err)
            source(err)
            from(e: std::num::ParseIntError) -> (Box::new(e))
            from(e: std::io::Error) -> (Box::new(e))
        }
        OtherMessage(s: String) {
            from()
            display("{}", s)
        }
        // ConsErrors(err: Box<Error>, tail: Box<Error>) {
        //     source(err)
        //     display("{}, {}", err, tail)
        // }
    }
}

impl Error {
    // pub fn append(self, new_err: Error) -> Error {
    //     Error::ConsErrors(Box::new(new_err), Box::new(self))
    // }

    pub fn is_need_more_data(&self) -> bool {
        if let Error::NeedMoreData(_n) = *self {
            return true;
        }
        false
    }

    // pub fn contains_need_more_data(&self) -> bool {
    //     if self.is_need_more_data() { return true;}
    //     // let head : Option<Box<Error>> = None;
    //     // let tail: Option<Box<Error>> = Some(self);
    //     if let &Error::ConsErrors(head, tail) = self {
    //         return head.contains_need_more_data_internal() || tail.contains_need_more_data_internal();
    //     }
    //     return false;
    // }

    // fn contains_need_more_data_internal(self: &Box<Error>) -> bool {
    //     let mut tail: &Box<Error> = self;
    //     loop {
    //         if tail.is_need_more_data() { return true;}
    //         if let &Error::ConsErrors(head, new_tail) = tail {
    //         }
    //     }

    // }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(v: std::sync::PoisonError<T>) -> Error {
        Error::OtherMessage(v.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type EmptyResult = Result<()>;

// /// Combine a result with an error.
// ///
// /// If the result already is an error, the new error gets appended.
// pub fn append_error(old: Result<()>, new_err: Error) -> Result<()> {
//     match old {
//         Err(old_e) => Err(old_e.append(new_err)),
//         Ok(()) => Err(new_err),
//     }
// }
