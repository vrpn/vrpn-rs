use std::{fmt::{self, Display}, ops::Add};

// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

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

    /// Maps `BytesRequired::Exactly(n)` to `BytesRequired::AtLeast(n)`
    pub fn expand(self) -> BytesRequired {
        match self {
            BytesRequired::Exactly(n) => BytesRequired::AtLeast(n),
            BytesRequired::AtLeast(n) => BytesRequired::AtLeast(n),
            BytesRequired::Unknown => BytesRequired::Unknown,
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

/// A minimal "error" indicating that an error did not contain a BytesRequired value.
pub struct DoesNotContainBytesRequired(());
