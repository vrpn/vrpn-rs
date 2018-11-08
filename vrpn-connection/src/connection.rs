// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    endpoint::Endpoint,
    translationtable::{Result as TranslationTableResult, TranslationTable, TranslationTableError},
    typedispatcher::{HandlerResult, MappingResult, RegisterMapping, TypeDispatcher},
};
use vrpn_base::{
    message::{Description, Message},
    types::*,
};

fn append_error(
    old: TranslationTableResult<()>,
    new_err: TranslationTableError,
) -> TranslationTableResult<()> {
    match old {
        Err(old_e) => Err(old_e.append(new_err)),
        Ok(()) => Err(new_err),
    }
}
pub trait Connection<'a> {
    /*
            disp.set_system_handler(constants::SENDER_DESCRIPTION, handle_sender_message);
            disp.set_system_handler(constants::TYPE_DESCRIPTION, handle_type_message);
            disp.set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
    */
    type EndpointItem: 'a + Endpoint;
    // type EndpointIterator: std::iter::Iterator<Item = &'a Option<Self::EndpointItem>>;
    // type EndpointIteratorMut: std::iter::Iterator<Item = &'a mut Option<Self::EndpointItem>>;

    // /// Get an iterator over the (mutable) endpoints
    // fn endpoints_iter_mut(&'a mut self) -> Self::EndpointIteratorMut;

    // /// Get an iterator over the endpoints.
    // fn endpoints_iter(&'a self) -> Self::EndpointIterator;

    /// Borrow a reference to the type dispatcher.
    // fn dispatcher(&'a self) -> &'a TypeDispatcher;

    // /// Borrow a mutable reference to the type dispatcher.
    // fn dispatcher_mut(&'a mut self) -> &'a mut TypeDispatcher;

    fn add_type(&mut self, name: TypeName) -> MappingResult<TypeId>;

    fn add_sender(&mut self, name: SenderName) -> MappingResult<SenderId>;

    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: &TypeName) -> Option<TypeId>;

    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: &SenderName) -> Option<SenderId>;

    fn pack_sender_description(
        &'a mut self,
        name: SenderName,
        sender: SenderId,
    ) -> TranslationTableResult<()> {
        let sender = LocalId(sender);
        let mut my_result = Ok(());
        for endpoint in self.endpoints_iter_mut().flatten() {
            match endpoint.pack_sender_description(sender) {
                Ok(()) => (),
                Err(e) => {
                    my_result = append_error(my_result, e);
                }
            }
            endpoint.new_local_sender(name.clone(), sender);
        }
        my_result
    }

    fn pack_type_description(
        &'a mut self,
        name: TypeName,
        message_type: TypeId,
    ) -> TranslationTableResult<()> {
        let message_type = LocalId(message_type);
        let mut my_result = Ok(());
        for endpoint in self.endpoints_iter_mut().flatten() {
            match endpoint.pack_type_description(message_type) {
                Ok(()) => (),
                Err(e) => {
                    my_result = append_error(my_result, e);
                }
            }
            endpoint.new_local_type(name.clone(), message_type);
        }
        my_result
    }

    fn register_sender(&'a mut self, name: SenderName) -> MappingResult<RegisterMapping<SenderId>>;

    fn register_type(&'a mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>>;
}
