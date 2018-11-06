// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    translationtable::{Result, TranslationTable, TranslationTableError},
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

    fn pack_sender_description(&mut self, local_sender: LocalId<SenderId>) -> Result<()>;

    fn pack_type_description(&mut self, local_type: LocalId<TypeId>) -> Result<()>;
}

pub trait Connection<'a> {
    /*
            disp.set_system_handler(constants::SENDER_DESCRIPTION, handle_sender_message);
            disp.set_system_handler(constants::TYPE_DESCRIPTION, handle_type_message);
            disp.set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
    */
    type EndpointItem: 'a + Endpoint;
    type EndpointIterator: std::iter::Iterator<Item = &'a Option<Self::EndpointItem>>;
    type EndpointIteratorMut: std::iter::Iterator<Item = &'a mut Option<Self::EndpointItem>>;

    /// Get an iterator over the (mutable) endpoints
    fn endpoints_iter_mut(&'a mut self) -> Self::EndpointIteratorMut;

    /// Get an iterator over the endpoints.
    fn endpoints_iter(&'a self) -> Self::EndpointIterator;

    /// Borrow a reference to the type dispatcher.
    fn dispatcher(&self) -> &TypeDispatcher;

    /// Borrow a mutable reference to the type dispatcher.
    fn dispatcher_mut(&mut self) -> &mut TypeDispatcher;

    fn add_type(&mut self, name: TypeName) -> MappingResult<TypeId>;

    fn add_sender(&mut self, name: SenderName) -> MappingResult<SenderId>;

    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: &TypeName) -> Option<TypeId>;

    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: &SenderName) -> Option<SenderId>;

    fn pack_sender_description(&'a mut self, name: SenderName, sender: SenderId) {
        let sender = LocalId(sender);
        for endpoint in self.endpoints_iter_mut().flatten() {
            endpoint.pack_sender_description(sender);
            endpoint.new_local_sender(name.clone(), sender);
        }
    }

    fn pack_type_description(&'a mut self, name: TypeName, message_type: TypeId) {
        let message_type = LocalId(message_type);
        for endpoint in self.endpoints_iter_mut().flatten() {
            endpoint.pack_type_description(message_type);
            endpoint.new_local_type(name.clone(), message_type);
        }
    }

    fn register_sender(&'a mut self, name: SenderName) -> MappingResult<RegisterMapping<SenderId>> {
        self.dispatcher_mut().register_sender(name)
    }

    fn register_type(&'a mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>> {
        self.dispatcher_mut().register_type(name)
    }
}

#[cfg(test)]
mod tests {
    use connection::*;
    #[test]
    fn log_names() {
        assert_eq!(make_log_name(None), None);
        assert_eq!(make_log_name(Some(String::from(""))), None);
        assert_eq!(
            make_log_name(Some(String::from("asdf"))),
            Some(String::from("asdf"))
        );
    }
}
