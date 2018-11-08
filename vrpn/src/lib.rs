// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate bytes;

extern crate socket2;

#[macro_use]
extern crate futures;

#[macro_use]
extern crate tokio;

extern crate pretty_hex;

#[macro_use]
extern crate quick_error;

extern crate vrpn_base;

extern crate vrpn_buffer;

extern crate vrpn_connection;

pub mod codec;
pub mod connect;
pub mod connection_ip;
pub(crate) mod endpoint_channel;
pub mod endpoint_ip;

pub use crate::connection_ip::ConnectionIP;

pub mod base {
    pub use vrpn_base::*;
}

pub mod buffer {
    pub use vrpn_buffer::*;
}

pub mod connection {
    pub use vrpn_connection::*;
}

pub use crate::base::constants;

pub mod prelude {
    pub use crate::buffer::prelude::*;
}
