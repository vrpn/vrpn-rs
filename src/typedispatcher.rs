// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use constants;
use std::error;
use std::fmt;
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

type HandlerInnerType = types::IdType;

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

/// Type storing a boxed callback function, an optional sender ID filter,
/// and the unique-per-CallbackCollection handle that can be used to unregister a handler.
///
/// Handler must live as long as the MsgCallbackEntry.
struct MsgCallbackEntry<'a> {
    handle: HandlerHandle,
    pub handler: Box<FnMut(&HandlerParams) -> HandlerResult<()> + 'a>,
    pub sender: IdToHandle<SenderId>,
}

impl<'a> MsgCallbackEntry<'a> {
    pub fn new<T: FnMut(&HandlerParams) -> HandlerResult<()> + 'a>(
        handle: HandlerHandle,
        handler: T,
        sender: IdToHandle<SenderId>,
    ) -> MsgCallbackEntry<'a> {
        MsgCallbackEntry {
            handle,
            handler: Box::new(handler),
            sender,
        }
    }
    /// Invokes the callback with the given params, if the sender filter (if not None) matches.
    pub fn call<'b>(&mut self, params: &'b HandlerParams) -> HandlerResult<()> {
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

/// Stores a collection of callbacks with a name, associated with either a message type,
/// or as a "global" handler mapping called for all message types.
struct CallbackCollection<'a> {
    name: String,
    callbacks: Vec<MsgCallbackEntry<'a>>,
    next_handle: HandlerInnerType,
}

impl<'a> CallbackCollection<'a> {
    /// Create CallbackCollection instance
    fn new(name: &str) -> CallbackCollection {
        CallbackCollection {
            name: String::from(name),
            callbacks: Vec::new(),
            next_handle: 0,
        }
    }

    /// Add a callback with optional sender ID filter
    fn add(
        &mut self,
        handler: Box<HandlerFnMut>,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        if self.callbacks.len() > types::MAX_VEC_USIZE {
            return Err(HandlerError::TooManyHandlers);
        }
        let handle = HandlerHandle(self.next_handle);
        self.callbacks
            .push(MsgCallbackEntry::new(handle, handler, sender));
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

    /// Call all callbacks (subject to sender filters)
    fn call(&mut self, params: &HandlerParams) -> HandlerResult<()> {
        for ref mut entry in self.callbacks.iter_mut() {
            entry.call(params)?;
        }
        Ok(())
    }
}

pub struct TypeDispatcher {
    types: Vec<CallbackCollection>,
    generic_callbacks: CallbackCollection,
    senders: Vec<String>,
    system_callbacks: Vec<Option<Box<HandlerFnMut>>>,
}

impl TypeDispatcher {
    pub fn create() -> HandlerResult<TypeDispatcher> {
        let mut disp = TypeDispatcher {
            types: Vec::new(),
            generic_callbacks: CallbackCollection::new("generic"),
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

    /// Get a mutable borrow of the CallbackCollection associated with the supplied TypeId
    /// (or the generic callbacks for AnyId)
    fn get_type_callbacks_mut(
        &mut self,
        type_id: IdToHandle<TypeId>,
    ) -> MappingResult<&mut CallbackCollection> {
        match type_id {
            SomeId(i) => {
                if i.0 < 0 {
                    return Err(MappingError::InvalidId);
                }
                let index = i.0 as usize;
                if index >= self.types.len() {
                    return Err(MappingError::InvalidId);
                }
                Ok(&mut self.types[index])
            }
            AnyId => Ok(&mut self.generic_callbacks),
        }
    }

    /// Get a borrow of the CallbackCollection associated with the supplied TypeId
    /// (or the generic callbacks for AnyId)
    fn get_type_callbacks(
        &self,
        type_id: IdToHandle<TypeId>,
    ) -> MappingResult<&CallbackCollection> {
        match type_id {
            SomeId(i) => {
                if i.0 < 0 {
                    return Err(MappingError::InvalidId);
                }
                let index = i.0 as usize;
                if index >= self.types.len() {
                    return Err(MappingError::InvalidId);
                }
                Ok(&self.types[index])
            }
            AnyId => Ok(&self.generic_callbacks),
        }
    }

    pub fn add_type(&mut self, name: &str) -> MappingResult<TypeId> {
        if self.types.len() > MAX_VEC_USIZE {
            return Err(MappingError::TooManyMappings);
        }
        self.types.push(CallbackCollection::new(name));
        Ok(TypeId((self.types.len() - 1) as IdType))
    }

    pub fn add_sender(&mut self, name: &str) -> MappingResult<SenderId> {
        if self.senders.len() > (IdType::max_value() - 2) as usize {
            return Err(MappingError::TooManyMappings);
        }
        self.senders.push(String::from(name));
        Ok(SenderId((self.senders.len() - 1) as IdType))
    }

    /// Returns the ID for the type name, if found.
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.types
            .iter()
            .position(|ref x| x.name == name)
            .map(|i| TypeId(i as IdType))
    }

    /// Calls add_type if get_type_id() returns None.
    /// Returns the corresponding TypeId in all cases.
    pub fn register_type(&mut self, name: &str) -> MappingResult<TypeId> {
        match self.get_type_id(name) {
            Some(i) => Ok(i),
            None => self.add_type(name),
        }
    }

    /// Calls add_sender if get_sender_id() returns None.
    pub fn register_sender(&mut self, name: &str) -> MappingResult<SenderId> {
        match self.get_sender_id(name) {
            Some(i) => Ok(i),
            None => self.add_sender(name),
        }
    }

    /// Returns the ID for the sender name, if found.
    pub fn get_sender_id(&self, name: &str) -> Option<SenderId> {
        let name_string = String::from(name);
        self.senders
            .iter()
            .position(|ref x| **x == name)
            .map(|i| SenderId(i as IdType))
    }

    pub fn add_handler<CB: 'static + FnMut(&HandlerParams) -> HandlerResult<()>>(
        &mut self,
        message_type: IdToHandle<TypeId>,
        cb: CB,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        self.get_type_callbacks_mut(message_type)?
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
        let type_index = message_type.0 as usize;
        if type_index >= self.types.len() {
            return Err(HandlerError::MappingErr(MappingError::InvalidId));
        }
        let ref mut mapping = &mut self.types[type_index];

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
#[cfg(test)]
mod tests {
    use typedispatcher::*;
    #[test]
    fn callback_collection() {
        let mut collection = CallbackCollection::new("dummy");
        let mut val = 5;
        let sample_callback = |params: &HandlerParams| -> HandlerResult<()> {
            val = 10;
            Ok(())
        };
        let handler = collection.add(Box::new(sample_callback), AnyId).unwrap();
        let params = HandlerParams {
            message_type: TypeId(0),
            sender: SenderId(0),
            msg_time: Time::default(),
            buffer: bytes::Bytes::with_capacity(10),
        };
        collection.call(&params);
        assert_eq!(val, 10);

        collection
            .remove(handler)
            .expect("Can't remove added callback");
        // No callbacks should fire now.
        val = 5;
        collection.call(&params);
        assert_eq!(val, 5);

        let sample_callback2 = |params: &HandlerParams| -> HandlerResult<()> {
            val = 15;
            Ok(())
        };
        let handler2 = collection
            .add(Box::new(sample_callback2), SomeId(SenderId(0)))
            .unwrap();
        val = 5;
        collection.call(&params);
        assert_eq!(val, 15);

        // Check that later-registered callbacks get run later
        let handler = collection.add(Box::new(sample_callback), AnyId).unwrap();
        val = 5;
        collection.call(&params);
        assert_eq!(val, 10);

        // This shouldn't trigger callback 2
        let params = HandlerParams {
            sender: SenderId(1),
            ..params
        };
        val = 5;
        collection.call(&params);
        assert_eq!(val, 10);
    }
}
