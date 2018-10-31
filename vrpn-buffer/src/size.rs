// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::fmt::{self, Display};
use std::ops::Add;

/// Optional trait for things that always take the same amount of space in a buffer.
///
/// Implementing this trait gets you implementations of a bunch of buffer/unbuffer-related traits for free.
pub trait ConstantBufferSize {
    /// Get the amount of space needed in a buffer.
    fn constant_buffer_size() -> usize;
}

#[derive(Debug)]
pub enum BytesRequired {
    Exactly(usize),
    AtLeast(usize),
    Unknown,
}

impl BytesRequired {
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
