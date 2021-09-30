// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

extern crate pin_project_lite;

pub mod bytes_mut_reader;
pub mod cookie;
pub mod endpoint_ip;
mod endpoints;
pub mod message_stream;
mod unbounded_message_sender;

use std::net::IpAddr;

use async_std::net::{TcpStream, ToSocketAddrs};
pub use message_stream::{AsyncReadMessagesExt, MessageStream};
pub(crate) use unbounded_message_sender::UnboundedMessageSender;

use crate::ServerInfo;

pub(crate) async fn connect_and_handshake(server_info: ServerInfo) -> crate::Result<TcpStream> {
    let mut stream = TcpStream::connect(server_info.socket_addr).await?;
    stream.set_nodelay(true)?;

    // We first write our cookie, then read and check the server's cookie, before the loop.
    cookie::send_nonfile_cookie(&mut stream).await?;
    cookie::read_and_check_nonfile_cookie(&mut stream).await?;
    Ok(stream)
}
