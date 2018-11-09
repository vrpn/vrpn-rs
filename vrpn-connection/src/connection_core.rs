// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::{Endpoint, Result, TranslationTable, TypeDispatcher};
use vrpn_base::{SenderId, TypeId};

pub struct ConnectionCore {
    dispatcher: TypeDispatcher,
}

impl ConnectionCore {
    pub fn new() -> Arc<Mutex<ConnectionCore>> {
        Arc::new(Mutex::new(ConnectionCore {
            dispatcher: TypeDispatcher::new(),
        }))
    }

    pub fn type_dispatcher_mut(&mut self) -> &mut TypeDispatcher {
        &mut self.dispatcher
    }
    pub fn type_dispatcher(&self) -> &TypeDispatcher {
        &self.dispatcher
    }
}
