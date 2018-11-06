// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    translationtable::{Result as TranslationTableResult, TranslationTable, TranslationTableError},
    typedispatcher::{HandlerResult, MappingResult, RegisterMapping, TypeDispatcher},
};
use vrpn_base::{
    message::{Description, Message},
    types::*,
};

#[derive(Debug, Clone)]
pub struct LogFileNames {
    pub in_log_file: Option<String>,
    pub out_log_file: Option<String>,
}

pub fn make_none_log_names() -> LogFileNames {
    LogFileNames {
        out_log_file: None,
        in_log_file: None,
    }
}

fn make_log_name(name: Option<String>) -> Option<String> {
    match name {
        None => None,
        Some(name_str) => {
            if name_str.len() > 0 {
                Some(name_str)
            } else {
                None
            }
        }
    }
}

pub fn make_log_names(log_names: Option<LogFileNames>) -> LogFileNames {
    match log_names {
        None => make_none_log_names(),
        Some(names) => LogFileNames {
            in_log_file: make_log_name(names.in_log_file),
            out_log_file: make_log_name(names.out_log_file),
        },
    }
}

pub trait Endpoint {
    fn send_message(
        &mut self,
        time: Time,
        message_type: TypeId,
        sender: SenderId,
        buffer: bytes::Bytes,
        class: ClassOfService,
    ) -> HandlerResult<()>;

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
