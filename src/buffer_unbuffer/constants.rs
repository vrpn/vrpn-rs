// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Constants that do not involve VRPN-specific data types.
//!
//! Constants in this file must remain unchanged so that they match the C++ implementation.

// This one might not go over the wire, so it might not be critical that it remain unchanged.
pub const GENERIC: &[u8] = b"generic";

pub const TCP_BUFLEN: usize = 64000;
pub const UDP_BUFLEN: usize = 1472;

/// "length of names in VRPN"
pub const CNAME_LEN: usize = 100;

/// default port to use
pub const DEFAULT_PORT: u16 = 3883;

pub const ALIGN: usize = 8;
