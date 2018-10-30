// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use cookie::Version;
use types::{SenderId, SenderName, TypeName};

// Constants in this file must remain unchanged so that they match the C++ implementation.
pub const GOT_FIRST_CONNECTION: TypeName = TypeName(b"VRPN_Connection_Got_First_Connection");
pub const GOT_CONNECTION: TypeName = TypeName(b"VRPN_Connection_Got_Connection");
pub const DROPPED_CONNECTION: TypeName = TypeName(b"VRPN_Connection_Dropped_Connection");
pub const DROPPED_LAST_CONNECTION: TypeName = TypeName(b"VRPN_Connection_Dropped_Last_Connection");

pub const CONTROL: SenderName = SenderName(b"VRPN Control");

// This one might not go over the wire, so it might not be critical that it remain unchanged.
pub const GENERIC: TypeName = TypeName(b"generic");

pub const SENDER_DESCRIPTION: SenderId = SenderId(-1);
pub const TYPE_DESCRIPTION: SenderId = SenderId(-2);
pub const UDP_DESCRIPTION: SenderId = SenderId(-3);
pub const LOG_DESCRIPTION: SenderId = SenderId(-4);
pub const DISCONNECT_MESSAGE: SenderId = SenderId(-5);

pub const TCP_BUFLEN: usize = 64000;
pub const UDP_BUFLEN: usize = 1472;

/// "length of names in VRPN"
pub const CNAME_LEN: usize = 100;

pub const MAGIC_PREFIX: &[u8] = b"vrpn: ver. ";
pub const MAGICLEN: usize = 16; // Must be a multiple of vrpn_ALIGN bytes!
pub const ALIGN: usize = 8;

// Based on vrpn_MAGIC_DATA
pub const MAGIC_DATA: Version = Version {
    major: 7,
    minor: 35,
};
pub const FILE_MAGIC_DATA: Version = Version { major: 4, minor: 0 };
//assert!(MAGICLEN % ALIGN == 0);

// NOTE: This needs to remain the same size unless we change the major version
// number for VRPN.  It is the length that is written into the stream.
pub const COOKIE_SIZE: usize = MAGICLEN + ALIGN;
