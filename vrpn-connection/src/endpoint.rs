// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::{Result, TranslationTable};
use downcast_rs::Downcast;
use vrpn_base::{SenderId, TypeId};

pub trait Endpoint: Downcast {
    /// Called by the system handler for UDP_DESCRIPTION messages.
    ///
    /// Default implementation is a no-op.
    fn connect_to_udp(&mut self, addr: Bytes, port: usize) -> Result<()> {
        Ok(())
    }

    fn type_table_mut(&mut self) -> &mut TranslationTable<TypeId>;
    fn sender_table_mut(&mut self) -> &mut TranslationTable<SenderId>;
}

impl_downcast!(Endpoint);
