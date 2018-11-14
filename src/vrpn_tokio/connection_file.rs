// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    descriptions::InnerDescription,
    type_dispatcher::{HandlerHandle, RegisterMapping},
    vrpn_tokio::endpoint_ip::EndpointIp,
    BaseTypeSafeId, Error, Handler, IdToHandle, LocalId, LogFileNames, MatchingTable, Message,
    MessageTypeIdentifier, Result, SenderId, SenderName, SomeId, StaticSenderName, StaticTypeName,
    TranslationTables, TypeDispatcher, TypeId, TypeName, TypedHandler, TypedMessageBody,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, TcpStream},
    prelude::*,
};