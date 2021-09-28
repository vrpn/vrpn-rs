// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Constants that relate to VRPN-specific data types.
//!
//! Constants in this file must remain unchanged so that they match the C++ implementation.

use crate::buffer_unbuffer::constants::ALIGN;

use super::{MessageTypeId, StaticMessageTypeName, StaticSenderName, Version};

pub const GOT_FIRST_CONNECTION: StaticMessageTypeName =
    StaticMessageTypeName(b"VRPN_Connection_Got_First_Connection");
pub const GOT_CONNECTION: StaticMessageTypeName =
    StaticMessageTypeName(b"VRPN_Connection_Got_Connection");
pub const DROPPED_CONNECTION: StaticMessageTypeName =
    StaticMessageTypeName(b"VRPN_Connection_Dropped_Connection");
pub const DROPPED_LAST_CONNECTION: StaticMessageTypeName =
    StaticMessageTypeName(b"VRPN_Connection_Dropped_Last_Connection");

pub const CONTROL: StaticSenderName = StaticSenderName(b"VRPN Control");

pub const SENDER_DESCRIPTION: MessageTypeId = MessageTypeId(-1);
pub const TYPE_DESCRIPTION: MessageTypeId = MessageTypeId(-2);
pub const UDP_DESCRIPTION: MessageTypeId = MessageTypeId(-3);
pub const LOG_DESCRIPTION: MessageTypeId = MessageTypeId(-4);
pub const DISCONNECT_MESSAGE: MessageTypeId = MessageTypeId(-5);

// Based on vrpn_MAGIC_DATA
pub const MAGIC_DATA: Version = Version {
    major: 7,
    minor: 35,
};
pub const FILE_MAGIC_DATA: Version = Version { major: 4, minor: 0 };

pub const MAGIC_PREFIX: &[u8] = b"vrpn: ver. ";
pub const MAGICLEN: usize = 16; // Must be a multiple of vrpn_ALIGN bytes!

/// This is the size, in bytes, of the "magic cookie" message.
///
/// NOTE: This needs to remain the same size unless we change the major version
/// number for VRPN.  It is the length that is written into the stream.
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
