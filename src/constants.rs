// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use types::SenderId;
use types::CookieData;

// Constants in this file must remain unchanged so that they match the C++ implementation.
pub const GOT_FIRST_CONNECTION: &str = "VRPN_Connection_Got_First_Connection";
pub const GOT_CONNECTION: &str = "VRPN_Connection_Got_Connection";
pub const DROPPED_CONNECTION: &str = "VRPN_Connection_Dropped_Connection";
pub const DROPPED_LAST_CONNECTION: &str = "VRPN_Connection_Dropped_Last_Connection";

pub const CONTROL: &str = "VRPN Control";

pub const SENDER_DESCRIPTION: SenderId = SenderId(-1);
pub const TYPE_DESCRIPTION: SenderId = SenderId(-2);
pub const UDP_DESCRIPTION: SenderId = SenderId(-3);
pub const LOG_DESCRIPTION: SenderId = SenderId(-4);
pub const DISCONNECT_MESSAGE: SenderId = SenderId(-5);

pub const TCP_BUFLEN: usize = 64000;
pub const UDP_BUFLEN: usize = 1472;

/// "length of names in VRPN"
pub const CNAME_LEN: usize = 100;

pub const MAGIC_PREFIX: &str = "vrpn: ver. ";
pub const MAGIC: &str = "vrpn: ver. 07.35";
pub const FILE_MAGIC: &str = "vrpn: ver. 04.00";
pub const MAGICLEN: usize = 16; // Must be a multiple of vrpn_ALIGN bytes!
pub const ALIGN: usize = 8;

pub const MAGIC_DATA: CookieData = CookieData{
    major: 7,
    minor: 35,
    log_mode: None,
};
pub const FILE_MAGIC_DATA: CookieData = CookieData{
    major: 4,
    minor: 0,
    log_mode: None,
};
//assert!(MAGICLEN % ALIGN == 0);

// NOTE: This needs to remain the same size unless we change the major version
// number for VRPN.  It is the length that is written into the stream.
pub const COOKIE_SIZE: usize = MAGICLEN + ALIGN;
