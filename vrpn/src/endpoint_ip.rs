// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use vrpn_base::{
    constants,
    message::{Description, Message},
    types::*,
};
use vrpn_connection::{
    connection::Endpoint, translationtable::TranslationTable, typedispatcher::HandlerResult,
};

use bytes::{BufMut, BytesMut};

pub struct EndpointIP {
    types: TranslationTable<TypeId>,
    senders: TranslationTable<SenderId>,
}

impl EndpointIP {
    pub fn new() -> EndpointIP {
        EndpointIP {
            types: TranslationTable::new(),
            senders: TranslationTable::new(),
        }
    }
}
impl Endpoint for EndpointIP {
    fn send_message(
        &mut self,
        time: Time,
        message_type: TypeId,
        sender: SenderId,
        buffer: bytes::Bytes,
        class: ClassOfService,
    ) -> HandlerResult<()> {
        unimplemented!();
    }

    fn local_type_id(&self, remote_type: RemoteId<TypeId>) -> Option<LocalId<TypeId>> {
        match self.types.map_to_local_id(remote_type) {
            Ok(val) => val,
            Err(_) => None,
        }
    }
    fn local_sender_id(&self, remote_sender: RemoteId<SenderId>) -> Option<LocalId<SenderId>> {
        match self.senders.map_to_local_id(remote_sender) {
            Ok(val) => val,
            Err(_) => None,
        }
    }

    fn new_local_sender(&mut self, name: SenderName, local_sender: LocalId<SenderId>) -> bool {
        self.senders.add_local_id(name.into(), local_sender)
    }

    fn new_local_type(&mut self, name: TypeName, local_type: LocalId<TypeId>) -> bool {
        self.types.add_local_id(name.into(), local_type)
    }

    fn pack_sender_description(&mut self, local_sender: LocalId<SenderId>) {
        let LocalId(sender) = local_sender;
        let entry = self.senders.get_by_local_id(local_sender).unwrap();
        let msg = Message::from(Description::new(sender, entry.name.clone()));
        // TODO do something with msg
    }

    fn pack_type_description(&mut self, local_type: LocalId<TypeId>) {
        // TODO handle negative types here, they're system message types.
        let LocalId(message_type) = local_type;
        let entry = self.types.get_by_local_id(local_type).unwrap();
        let msg = Message::from(Description::new(message_type, entry.name.clone()));
        // TODO do something with msg
    }
}
