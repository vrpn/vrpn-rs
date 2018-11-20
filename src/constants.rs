// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    cookie::Version,
    types::{StaticSenderName, StaticTypeName, TypeId},
};

// Constants in this file must remain unchanged so that they match the C++ implementation.
pub const GOT_FIRST_CONNECTION: StaticTypeName =
    StaticTypeName(b"VRPN_Connection_Got_First_Connection");
pub const GOT_CONNECTION: StaticTypeName = StaticTypeName(b"VRPN_Connection_Got_Connection");
pub const DROPPED_CONNECTION: StaticTypeName =
    StaticTypeName(b"VRPN_Connection_Dropped_Connection");
pub const DROPPED_LAST_CONNECTION: StaticTypeName =
    StaticTypeName(b"VRPN_Connection_Dropped_Last_Connection");

pub const CONTROL: StaticSenderName = StaticSenderName(b"VRPN Control");

// This one might not go over the wire, so it might not be critical that it remain unchanged.
pub const GENERIC: &[u8] = b"generic";

pub const SENDER_DESCRIPTION: TypeId = TypeId(-1);
pub const TYPE_DESCRIPTION: TypeId = TypeId(-2);
pub const UDP_DESCRIPTION: TypeId = TypeId(-3);
pub const LOG_DESCRIPTION: TypeId = TypeId(-4);
pub const DISCONNECT_MESSAGE: TypeId = TypeId(-5);

pub const TCP_BUFLEN: usize = 64000;
pub const UDP_BUFLEN: usize = 1472;

/// "length of names in VRPN"
pub const CNAME_LEN: usize = 100;

/// default port to use
pub const DEFAULT_PORT: u16 = 3883;

pub const MAGIC_PREFIX: &[u8] = b"vrpn: ver. ";
pub const MAGICLEN: usize = 16; // Must be a multiple of vrpn_ALIGN bytes!
pub const ALIGN: usize = 8;

// Based on vrpn_MAGIC_DATA
pub const MAGIC_DATA: Version = Version {
    major: 7,
    minor: 35,
};
pub const FILE_MAGIC_DATA: Version = Version { major: 4, minor: 0 };

// NOTE: This needs to remain the same size unless we change the major version
// number for VRPN.  It is the length that is written into the stream.
pub const COOKIE_SIZE: usize = MAGICLEN + ALIGN;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn magiclen() {
        assert_eq!(MAGICLEN % ALIGN, 0);
        assert_eq!(COOKIE_SIZE % ALIGN, 0);
    }
}
