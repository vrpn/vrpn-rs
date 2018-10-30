// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use endpoint_ip::EndpointIP;
use typedispatcher::{HandlerResult, MappingResult};
use types::*;

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

    fn new_local_sender(&mut self, name: SenderName, local_sender: LocalId<SenderId>) -> bool;
    fn new_local_type(&mut self, name: TypeName, local_type: LocalId<TypeId>) -> bool;

    fn pack_sender_description(&mut self, local_sender: LocalId<SenderId>);
    fn pack_type_description(&mut self, local_type: LocalId<TypeId>);
}

pub trait Connection {
    /*
            disp.set_system_handler(constants::SENDER_DESCRIPTION, handle_sender_message);
            disp.set_system_handler(constants::TYPE_DESCRIPTION, handle_type_message);
            disp.set_system_handler(constants::DISCONNECT_MESSAGE, handle_disconnect_message);
    */
    fn add_type(&mut self, name: TypeName) -> MappingResult<TypeId>;

    fn add_sender(&mut self, name: SenderName) -> MappingResult<SenderId>;

    /// Returns the ID for the type name, if found.
    fn get_type_id(&self, name: &TypeName) -> Option<TypeId>;
    /// Returns the ID for the sender name, if found.
    fn get_sender_id(&self, name: &SenderName) -> Option<SenderId>;
    fn call_on_each_mut_endpoint<'a, F: 'a + FnMut(&mut dyn Endpoint)>(&'a mut self, f: F);
    fn call_on_each_endpoint<'a, F: 'a + Fn(&dyn Endpoint)>(&self, f: F);
    /*
    fn endpoints_iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut impl Endpoint>;
    fn endpoints_iter<'a>(&'a mut self) -> impl Iterator<Item = &dyn Endpoint>;
    */
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

    fn register_sender(&mut self, name: SenderName) -> MappingResult<SenderId> {
        match self.get_sender_id(&name) {
            Some(id) => Ok(id),
            None => {
                let sender = self.add_sender(name.clone())?;
                self.pack_sender_description(sender);
                self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
                    e.new_local_sender(name.clone(), LocalId(sender));
                });
                Ok(sender)
            }
        }
    }
    fn register_type(&mut self, name: TypeName) -> MappingResult<TypeId> {
        match self.get_type_id(&name) {
            Some(id) => Ok(id),
            None => {
                let message_type = self.add_type(name.clone())?;
                self.pack_type_description(message_type);
                self.call_on_each_mut_endpoint(|e: &mut dyn Endpoint| {
                    e.new_local_type(name.clone(), LocalId(message_type));
                });
                Ok(message_type)
            }
        }
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
