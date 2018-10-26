// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::error;
use std::fmt;
use constants;
use types::*;
extern crate bytes;

#[derive(Debug, Clone)]
pub enum MappingError {
    TooManyMappings,
    InvalidId,
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MappingError::TooManyMappings => write!(f, "too many mappings"),
            MappingError::InvalidId => write!(f, "invalid id"),
        }
    }
}

impl error::Error for MappingError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum HandlerError {
    TooManyHandlers,
    HandlerNotFound,
    MappingErr(MappingError),
    GenericErrorReturn,
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HandlerError::TooManyHandlers => write!(f, "too many handlers"),
            HandlerError::MappingErr(e) => write!(f, "{}", e),
            HandlerError::HandlerNotFound => write!(f, "handler not found"),
            HandlerError::GenericErrorReturn => write!(f, "handler returned an error"),
        }
    }
}

impl error::Error for HandlerError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            HandlerError::MappingErr(e) => Some(e),
            _ => None,
        }
    }
}

type HandlerInnerType = u32;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HandlerHandle(HandlerInnerType);

pub type MappingResult<T> = Result<T, MappingError>;

pub type HandlerResult<T> = Result<T, HandlerError>;

impl From<MappingError> for HandlerError {
    fn from(e: MappingError) -> HandlerError {
        HandlerError::MappingErr(e)
    }
}
type HandlerFnMut = FnMut(&HandlerParams) -> HandlerResult<()>;

struct MsgCallbackEntry {
    handle: HandlerHandle,
    pub handler: Box<HandlerFnMut>,
    pub sender: IdToHandle<SenderId>,
}
impl MsgCallbackEntry {
    pub fn call(&mut self, params: &HandlerParams) -> HandlerResult<()> {
        let should_call = match self.sender {
            AnyId => true,
            SomeId(i) => i == params.sender,
        };
        if should_call {
            (self.handler)(params)
        } else {
            Ok(())
        }
    }
}

struct LocalMapping {
    name: String,
    callbacks: Vec<MsgCallbackEntry>,
    next_handle: HandlerInnerType,
}

impl LocalMapping {
    fn new(name: &str) -> LocalMapping {
        LocalMapping {
            name: String::from(name),
            callbacks: Vec::new(),
            next_handle: 0,
        }
    }

    /// Add a callback
    fn add(
        &mut self,
        handler: Box<HandlerFnMut>,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        if self.callbacks.len() > (HandlerInnerType::max_value() - 2) as usize {
            return Err(HandlerError::TooManyHandlers);
        }
        let handle = HandlerHandle(self.next_handle);
        self.callbacks.push(MsgCallbackEntry {
            handle,
            handler,
            sender,
        });
        self.next_handle += 1;
        Ok(handle)
    }

    /// Remove a callback
    fn remove(&mut self, handle: HandlerHandle) -> HandlerResult<()> {
        match self.callbacks.iter().position(|ref x| x.handle == handle) {
            Some(i) => {
                self.callbacks.remove(i);
                Ok(())
            }
            None => Err(HandlerError::HandlerNotFound),
        }
    }

    /// Call all callbacks
    fn call(&mut self, params: &HandlerParams) -> HandlerResult<()> {
        for ref mut entry in self.callbacks.iter_mut() {
            match entry.call(params) {
                Err(e) => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }
}

struct LocalMappingCollection {
    mappings: Vec<LocalMapping>,
}

impl LocalMappingCollection {
    fn new() -> LocalMappingCollection {
        LocalMappingCollection {
            mappings: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.mappings.len()
    }

    fn get_mut(&mut self, id: IdType) -> MappingResult<&mut LocalMapping> {
        if id < 0 || id as usize >= self.mappings.len() {
            Err(MappingError::InvalidId)
        } else {
            Ok(&mut self.mappings[id as usize])
        }
    }

    fn get(&self, id: IdType) -> MappingResult<&LocalMapping> {
        if id < 0 || id as usize >= self.mappings.len() {
            Err(MappingError::InvalidId)
        } else {
            Ok(&self.mappings[id as usize])
        }
    }

    /// Unconditionally adds, returning the ID
    fn add(&mut self, name: &str) -> MappingResult<IdType> {
        if self.mappings.len() > (IdType::max_value() - 2) as usize {
            return Err(MappingError::TooManyMappings);
        }
        self.mappings.push(LocalMapping::new(name));
        Ok((self.mappings.len() - 1) as IdType)
    }

    /// Returns the ID for the name, if found.
    fn get_by_name(&self, name: &str) -> Option<IdType> {
        self.mappings
            .iter()
            .position(|ref x| x.name == name)
            .map(|i| i as IdType)
    }

    /// Conditionally adds (if not already added), returning ID.
    fn register(&mut self, name: &str) -> MappingResult<IdType> {
        match self.get_by_name(name) {
            Some(i) => Ok(i),
            None => self.add(name),
        }
    }

    fn add_handler(
        &mut self,
        id: IdType,
        handler: Box<HandlerFnMut>,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        self.get_mut(id)?.add(handler, sender)
    }

    fn remove_handler(&mut self, id: IdType, handle: HandlerHandle) -> HandlerResult<()> {
        self.get_mut(id)?.remove(handle)
    }
}

pub struct TypeDispatcher {
    types: LocalMappingCollection,
    generic_callbacks: LocalMapping,
    senders: Vec<String>,
    system_callbacks: Vec<Option<Box<HandlerFnMut>>>,
}

impl TypeDispatcher {
    pub fn create() -> HandlerResult<TypeDispatcher> {
        let mut disp = TypeDispatcher {
            types: LocalMappingCollection::new(),
            generic_callbacks: LocalMapping::new("generic"),
            senders: Vec::new(),
            system_callbacks: Vec::new(),
        };

        disp.register_sender(constants::CONTROL)?;
        disp.register_type(constants::GOT_FIRST_CONNECTION)?;
        disp.register_type(constants::GOT_CONNECTION)?;
        disp.register_type(constants::DROPPED_CONNECTION)?;
        disp.register_type(constants::DROPPED_LAST_CONNECTION)?;
        Ok(disp)
    }

    fn get_mapping_mut(&mut self, type_id: IdToHandle<TypeId>) -> MappingResult<&mut LocalMapping> {
        match type_id {
            SomeId(i) => self.types.get_mut(i.0),
            AnyId => Ok(&mut self.generic_callbacks),
        }
    }

    fn get_mapping(&self, type_id: IdToHandle<TypeId>) -> MappingResult<&LocalMapping> {
        match type_id {
            SomeId(i) => self.types.get(i.0),
            AnyId => Ok(&self.generic_callbacks),
        }
    }

    pub fn add_type(&mut self, name: &str) -> MappingResult<TypeId> {
        self.types.add(name).map(|i| TypeId(i))
    }

    pub fn add_sender(&mut self, name: &str) -> MappingResult<SenderId> {
        if self.senders.len() > (IdType::max_value() - 2) as usize {
            return Err(MappingError::TooManyMappings);
        }
        self.senders.push(String::from(name));
        Ok(SenderId((self.senders.len() - 1) as IdType))
    }

    /// Calls add_type if get_type_id() returns None.
    pub fn register_type(&mut self, name: &str) -> MappingResult<TypeId> {
        self.types.register(name).map(|i| TypeId(i))
    }

    /// Calls add_sender if get_sender_id() returns None.
    pub fn register_sender(&mut self, name: &str) -> MappingResult<SenderId> {
        match self.get_sender_id(name) {
            Some(i) => Ok(i),
            None => self.add_sender(name),
        }
    }

    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.types.get_by_name(name).map(|i| TypeId(i))
    }

    pub fn get_sender_id(&self, name: &str) -> Option<SenderId> {
        let name_string = String::from(name);
        self.senders
            .iter()
            .position(|ref x| **x == name_string)
            .map(|i| SenderId(i as IdType))
    }

    pub fn add_handler<CB: 'static + FnMut(&HandlerParams) -> HandlerResult<()>>(
        &mut self,
        message_type: IdToHandle<TypeId>,
        cb: CB,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        self.get_mapping_mut(message_type)?
            .add(Box::new(cb), sender)
    }

    pub fn do_callbacks_for(
        &mut self,
        message_type: TypeId,
        sender: SenderId,
        msg_time: Time,
        buffer: bytes::Bytes,
    ) -> HandlerResult<()> {
        // Don't dispatch system messages here
        if message_type.0 < 0 {
            return Ok(());
        }

        // This will early-return if the message type is bad.
        let ref mut mapping = self.types.get_mut(message_type.0)?;

        let params = HandlerParams {
            message_type,
            sender,
            msg_time,
            buffer,
        };

        self.generic_callbacks.call(&params)?;
        mapping.call(&params)
    }

    pub fn do_system_callbacks_for(
        &mut self,
        message_type: TypeId,
        sender: SenderId,
        msg_time: Time,
        buffer: bytes::Bytes,
    ) -> HandlerResult<()> {
        if message_type.0 >= 0 {
            return Err(HandlerError::MappingErr(MappingError::InvalidId));
        }
        let real_index = (-message_type.0) as usize;
        if real_index >= self.system_callbacks.len() {
            // Not an error to try to call an unhandled system message
            return Ok(());
        }
        match self.system_callbacks[real_index] {
            Some(ref mut handler) => {
                let params = HandlerParams {
                    message_type,
                    sender,
                    msg_time,
                    buffer,
                };
                (handler)(&params)
            }
            None => Ok(()),
        }
    }

    pub fn set_system_handler<CB: 'static + FnMut(&HandlerParams) -> HandlerResult<()>>(
        &mut self,
        message_type: TypeId,
        handler: CB,
    ) -> HandlerResult<()> {
        if message_type.0 >= 0 {
            return Err(HandlerError::MappingErr(MappingError::InvalidId));
        }
        let real_index = (-message_type.0) as usize;
        while real_index >= self.system_callbacks.len() {
            self.system_callbacks.push(None);
        }
        self.system_callbacks[real_index] = Some(Box::new(handler));
        Ok(())
    }
}
