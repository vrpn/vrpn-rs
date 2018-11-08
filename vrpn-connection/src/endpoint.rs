// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    translationtable::{Result as TranslationTableResult, TranslationTable},
    typedispatcher::HandlerResult,
};
use vrpn_base::{
    message::{GenericMessage, Message},
    types::*,
};
use vrpn_buffer::{message::make_message_body_generic, Buffer};

pub trait Endpoint {
    fn buffer_generic_message(
        &mut self,
        msg: GenericMessage,
        class: ClassOfService,
    ) -> HandlerResult<()>;

    fn buffer_message<T: Buffer>(
        &mut self,
        msg: Message<T>,
        class: ClassOfService,
    ) -> HandlerResult<()> {
        let generic_msg = make_message_body_generic(msg)?;
        self.buffer_generic_message(generic_msg, class)
    }
    /// Borrow a reference to the translation table of sender IDs
    fn sender_table(&self) -> &TranslationTable<SenderId>;

    /// Borrow a mutable reference to the translation table of sender IDs
    fn sender_table_mut(&mut self) -> &mut TranslationTable<SenderId>;

    /// Borrow a reference to the translation table of type IDs
    fn type_table(&self) -> &TranslationTable<TypeId>;

    /// Borrow a mutable reference to the translation table of type IDs
    fn type_table_mut(&mut self) -> &mut TranslationTable<TypeId>;

    /// Convert remote type ID to local type ID
    fn local_type_id(&self, remote_type: RemoteId<TypeId>) -> Option<LocalId<TypeId>> {
        match self.type_table().map_to_local_id(remote_type) {
            Ok(val) => val,
            Err(_) => None,
        }
    }

    /// Convert remote sender ID to local sender ID
    fn local_sender_id(&self, remote_sender: RemoteId<SenderId>) -> Option<LocalId<SenderId>> {
        match self.sender_table().map_to_local_id(remote_sender) {
            Ok(val) => val,
            Err(_) => None,
        }
    }

    fn new_local_sender(&mut self, name: SenderName, local_sender: LocalId<SenderId>) -> bool {
        self.sender_table_mut()
            .add_local_id(name.into(), local_sender)
    }

    fn new_local_type(&mut self, name: TypeName, local_type: LocalId<TypeId>) -> bool {
        self.type_table_mut().add_local_id(name.into(), local_type)
    }

    fn pack_sender_description(
        &mut self,
        local_sender: LocalId<SenderId>,
    ) -> TranslationTableResult<()>;

    fn pack_type_description(&mut self, local_type: LocalId<TypeId>) -> TranslationTableResult<()>;
}
