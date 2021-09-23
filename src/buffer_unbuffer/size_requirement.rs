// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::{
    fmt::{self, Display},
    ops::Add,
    result,
};

/// Implemented by types (like errors) that may contain a `SizeRequirement`.
///
/// Not to be confused with ```BufferSize```, which is for types that can tell
/// you the size they would take up in a buffer.
pub trait MayContainSizeRequirement {
    fn try_get_size_requirement(self) -> Option<SizeRequirement>;
}

// pub trait GetContainsSizeRequirement: MayContainSizeRequirement {
//     fn contains_size_requirement(&self) -> bool;
// }
// impl<'a, T> GetContainsSizeRequirement for T
// where
//     T: 'a + MayContainSizeRequirement,
//     &'a T: MayContainSizeRequirement,
// {
//     fn contains_size_requirement(&self) -> bool {
//         self.try_get_size_requirement().is_some()
//     }
// }

/// Implemented by types (like errors) that may contain a `SizeRequirement`,
/// and that can be consumed expanding their size requirement if present.
pub trait ExpandSizeRequirement /* : MayContainSizeRequirement */ {
    fn expand_size_requirement(self) -> Self;
}

// impl<T, E: MayContainSizeRequirement> MayContainSizeRequirement for result::Result<T, E> {
impl<T, E: ExpandSizeRequirement> ExpandSizeRequirement for result::Result<T, E> {
    fn expand_size_requirement(self) -> Self {
        match self {
            Err(e) => Err(e.expand_size_requirement()),
            _ => self,
        }
    }
}

/// Expresses how many more bytes we require/expect when parsing a message.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeRequirement {
    Exactly(usize),
    AtLeast(usize),
    Unknown,
}

impl SizeRequirement {
    /// Compares a size requirement to available bytes in a buffer.
    ///
    /// Note that in this case, Exactly(c) is satisfied by c or anything larger.
    pub fn satisfied_by(&self, buf_size: usize) -> Option<bool> {
        match *self {
            SizeRequirement::Exactly(c) => Some(c <= buf_size),
            SizeRequirement::AtLeast(c) => Some(c <= buf_size),
            SizeRequirement::Unknown => None,
        }
    }

    /// Maps `SizeRequirement::Exactly(n)` to `SizeRequirement::AtLeast(n)`
    pub fn expand(self) -> SizeRequirement {
        match self {
            SizeRequirement::Exactly(n) => SizeRequirement::AtLeast(n),
            SizeRequirement::AtLeast(n) => SizeRequirement::AtLeast(n),
            SizeRequirement::Unknown => SizeRequirement::Unknown,
        }
    }
}

impl MayContainSizeRequirement for SizeRequirement {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        Some(self)
    }
}

impl MayContainSizeRequirement for &SizeRequirement {
    fn try_get_size_requirement(self) -> Option<SizeRequirement> {
        Some(*self)
    }
}

impl ExpandSizeRequirement for SizeRequirement {
    fn expand_size_requirement(self) -> Self {
        self.expand()
    }
}

impl Add for SizeRequirement {
    type Output = SizeRequirement;
    fn add(self, other: SizeRequirement) -> Self::Output {
        use self::SizeRequirement::*;

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

impl Display for SizeRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SizeRequirement::Exactly(n) => write!(f, "exactly {}", n),
            SizeRequirement::AtLeast(n) => write!(f, "at least {}", n),
            SizeRequirement::Unknown => write!(f, "unknown"),
        }
    }
}

/// A minimal "error" indicating that an error did not contain a BytesRequired value.
pub struct DoesNotContainBytesRequired(());
