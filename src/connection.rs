// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use endpoint_ip::EndpointIP;
use typedispatcher::HandlerResult;
use typedispatcher::MappingResult;
use typedispatcher::TypeDispatcher;
use types::*;
extern crate bytes;

#[derive(Debug, Clone)]
pub struct LogFileNames {
    pub in_log_file: Option<String>,
    pub out_log_file: Option<String>,
}

pub trait EndpointAllocator {
    fn allocate(&self) -> Option<Box<EndpointIP>> {
        None
    }
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

    fn local_type_id(&self, remote_type: RemoteId<TypeId>) -> Option<LocalId<TypeId>>;
    fn local_sender_id(&self, remote_sender: RemoteId<SenderId>) -> Option<LocalId<SenderId>>;

    fn new_local_sender(&mut self, name: &'static str, local_sender: LocalId<SenderId>) -> bool;
    fn new_local_type(&mut self, name: &'static str, local_type: LocalId<TypeId>) -> bool;

    fn pack_sender_description(&mut self, local_sender: LocalId<SenderId>);
    fn pack_type_description(&mut self, local_type: LocalId<TypeId>);
}

pub trait Connection {
    /*
        disp.set_system_handler(constants::SENDER_DESCRIPTION, handle_sender_message);
        disp.set_system_handler(constants::TYPE_DESCRIPTION, handle_type_message);
        disp.set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
*/
    fn dispatcher_mut(&mut self) -> &mut TypeDispatcher;
    fn dispatcher(&self) -> &TypeDispatcher;
    fn call_on_each_mut_endpoint<'a, F: 'a + FnMut(&mut dyn Endpoint)>(&'a mut self, f: F);
    fn call_on_each_endpoint<'a, F: 'a + Fn(&dyn Endpoint)>(&self, f: F);
    fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.dispatcher().get_type_id(name)
    }
    
    fn get_sender_id(&self, name: &str) -> Option<SenderId> {
        self.dispatcher().get_sender_id(name)
    }

    fn pack_sender_description(&mut self, sender: SenderId) {
        self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
            e.pack_sender_description(LocalId(sender))
        })
    }

    fn pack_type_description(&mut self, message_type: TypeId) {
        self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
            e.pack_type_description(LocalId(message_type))
        })
    }

    fn register_sender(&mut self, name: &'static str) -> MappingResult<SenderId> {
        match self.get_sender_id(name) {
            Some(id) => Ok(id),
            None => {
                let sender = self.dispatcher_mut().add_sender(name)?;
                self.pack_sender_description(sender);
                self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
                    e.new_local_sender(&name, LocalId(sender));
                });
                Ok(sender)
            }
        }
    }
    fn register_type(&mut self, name: &'static str) -> MappingResult<TypeId> {
        match self.get_type_id(name) {
            Some(id) => Ok(id),
            None => {
                let message_type = self.dispatcher_mut().add_type(name)?;
                self.pack_type_description(message_type);
                self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
                    e.new_local_type(&name, LocalId(message_type));
                });
                Ok(message_type)
            }
        }
    }
}
