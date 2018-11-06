// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;

extern crate socket2;

#[macro_use]
extern crate tokio;

#[macro_use]
extern crate quick_error;

extern crate vrpn_base;

extern crate vrpn_buffer;

extern crate vrpn_connection;

pub mod connection_ip;
pub mod endpoint_ip;
pub mod error;
pub mod vrpn_tokio;

pub use connection_ip::ConnectionIP;

pub mod base {
    pub use vrpn_base::*;
}

pub mod buffer {
    pub use vrpn_buffer::*;
}

pub mod connection {
    pub use vrpn_connection::*;
}

pub use base::constants;

pub mod prelude {
    pub use buffer::prelude::*;
}
