// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use vrpn_base::{
    constants,
    message::GenericMessage,
    types::{self, *},
};
use vrpn_buffer::buffer;

quick_error! {
    #[derive(Debug)]
    pub enum MappingError {
        TooManyMappings {
            description("too many mappings")
        }
        InvalidId{
            description("invalid id")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum HandlerError {
        TooManyHandlers {
            description("too many handlers")
        }
        HandlerNotFound {
            description("handler not found")
        }
        MappingErr(err: MappingError) {
            from()
            cause(err)
            display("{}", err)
        }
        GenericErrorReturn {
            description("handler returned an error")
        }
        BufferError(err: buffer::Error) {
            from()
            cause(err)
            display("{}", err)
        }
    }
}

pub enum RegisterMapping<T: BaseTypeSafeId> {
    /// This was an existing mapping with the given ID
    Found(T),
    /// This was a new mapping, which has been registered and received the given ID
    NewMapping(T),
}

type HandlerInnerType = types::IdType;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HandlerHandle(HandlerInnerType);

pub type MappingResult<T> = Result<T, MappingError>;

pub type HandlerResult<T> = Result<T, HandlerError>;

type HandlerFnMut = FnMut(&GenericMessage) -> HandlerResult<()>;

/// Type storing a boxed callback function, an optional sender ID filter,
/// and the unique-per-CallbackCollection handle that can be used to unregister a handler.
///
/// Handler must live as long as the MsgCallbackEntry.
struct MsgCallbackEntry<'a> {
    handle: HandlerHandle,
    pub handler: Box<FnMut(&GenericMessage) -> HandlerResult<()> + 'a>,
    pub sender: IdToHandle<SenderId>,
}

impl<'a> MsgCallbackEntry<'a> {
    pub fn new<T: FnMut(&GenericMessage) -> HandlerResult<()> + 'a>(
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
    pub fn call<'b>(&mut self, msg: &'b GenericMessage) -> HandlerResult<()> {
        let should_call = match self.sender {
            AnyId => true,
            SomeId(i) => i == msg.header.sender,
        };
        if should_call {
            (self.handler)(msg)
        } else {
            Ok(())
        }
    }
}

/// Stores a collection of callbacks with a name, associated with either a message type,
/// or as a "global" handler mapping called for all message types.
struct CallbackCollection<'a> {
    name: TypeName<'a>,
    callbacks: Vec<MsgCallbackEntry<'a>>,
    next_handle: HandlerInnerType,
}

impl<'a> CallbackCollection<'a> {
    /// Create CallbackCollection instance
    pub fn new(name: TypeName) -> CallbackCollection<'a> {
        CallbackCollection {
            name,
            callbacks: Vec::new(),
            next_handle: 0,
        }
    }

    /// Add a callback with optional sender ID filter
    fn add<T: FnMut(&GenericMessage) -> HandlerResult<()> + 'a>(
        &mut self,
        handler: T,
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
    fn call(&mut self, params: &GenericMessage) -> HandlerResult<()> {
        for ref mut entry in self.callbacks.iter_mut() {
            entry.call(params)?;
        }
        Ok(())
    }
}

pub struct TypeDispatcher<'a> {
    types: Vec<CallbackCollection<'a>>,
    generic_callbacks: CallbackCollection<'a>,
    senders: Vec<SenderName>,
    system_callbacks: Vec<Option<Box<HandlerFnMut>>>,
}

impl<'a> TypeDispatcher<'a> {
    pub fn new() -> HandlerResult<TypeDispatcher<'a>> {
        let mut disp = TypeDispatcher {
            types: Vec::new(),
            generic_callbacks: CallbackCollection::new(constants::GENERIC),
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
        &'a mut self,
        type_id: IdToHandle<TypeId>,
    ) -> MappingResult<&'a mut CallbackCollection> {
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
        &'a self,
        type_id: IdToHandle<TypeId>,
    ) -> MappingResult<&'a CallbackCollection> {
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

    pub fn add_type(&mut self, name: TypeName) -> MappingResult<TypeId> {
        if self.types.len() > MAX_VEC_USIZE {
            return Err(MappingError::TooManyMappings);
        }
        self.types.push(CallbackCollection::new(name));
        Ok(TypeId((self.types.len() - 1) as IdType))
    }

    pub fn add_sender(&mut self, name: SenderName) -> MappingResult<SenderId> {
        if self.senders.len() > (IdType::max_value() - 2) as usize {
            return Err(MappingError::TooManyMappings);
        }
        self.senders.push(name);
        Ok(SenderId((self.senders.len() - 1) as IdType))
    }

    /// Returns the ID for the type name, if found.
    pub fn get_type_id(&self, name: &TypeName) -> Option<TypeId> {
        self.types
            .iter()
            .position(|ref x| x.name == *name)
            .map(|i| TypeId(i as IdType))
    }

    /// Calls add_type if get_type_id() returns None.
    /// Returns the corresponding TypeId in all cases.
    pub fn register_type(&mut self, name: TypeName) -> MappingResult<RegisterMapping<TypeId>> {
        match self.get_type_id(&name) {
            Some(i) => Ok(RegisterMapping::Found(i)),
            None => self.add_type(name).map(|i| RegisterMapping::NewMapping(i)),
        }
    }

    /// Calls add_sender if get_sender_id() returns None.
    pub fn register_sender(
        &mut self,
        name: SenderName,
    ) -> MappingResult<RegisterMapping<SenderId>> {
        match self.get_sender_id(&name) {
            Some(i) => Ok(RegisterMapping::Found(i)),
            None => self
                .add_sender(name)
                .map(|i| RegisterMapping::NewMapping(i)),
        }
    }

    /// Returns the ID for the sender name, if found.
    pub fn get_sender_id(&self, name: &SenderName) -> Option<SenderId> {
        self.senders
            .iter()
            .position(|ref x| *x == name)
            .map(|i| SenderId(i as IdType))
    }

    pub fn add_handler<CB: 'a + FnMut(&GenericMessage) -> HandlerResult<()>>(
        &'a mut self,
        message_type: IdToHandle<TypeId>,
        cb: CB,
        sender: IdToHandle<SenderId>,
    ) -> HandlerResult<HandlerHandle> {
        self.get_type_callbacks_mut(message_type)?.add(cb, sender)
    }

    pub fn do_callbacks_for(&mut self, msg: GenericMessage) -> HandlerResult<()> {
        let raw_message_type = msg.header.message_type.0;
        // Don't dispatch system messages here
        if raw_message_type < 0 {
            return Ok(());
        }
        let type_index = raw_message_type as usize;
        if type_index >= self.types.len() {
            return Err(HandlerError::MappingErr(MappingError::InvalidId));
        }
        let ref mut mapping = &mut self.types[type_index];

        self.generic_callbacks.call(&msg)?;
        mapping.call(&msg)
    }

    pub fn do_system_callbacks_for(&mut self, msg: GenericMessage) -> HandlerResult<()> {
        if msg.header.message_type.0 >= 0 {
            return Err(HandlerError::MappingErr(MappingError::InvalidId));
        }
        let real_index = (-msg.header.message_type.0) as usize;
        if real_index >= self.system_callbacks.len() {
            // Not an error to try to call an unhandled system message
            return Ok(());
        }
        match self.system_callbacks[real_index] {
            Some(ref mut handler) => (handler)(&msg),
            None => Ok(()),
        }
    }

    pub fn set_system_handler<CB: 'static + FnMut(&GenericMessage) -> HandlerResult<()>>(
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
    use crate::typedispatcher::*;
    use std::rc::Rc;
    #[test]
    fn callback_collection() {
        /*
        let val: Rc<i8> = Rc::new(5);
        let a = Rc::clone(&val);
        let mut sample_callback = |params: &GenericMessage| -> HandlerResult<()> {
            a = 10;
            Ok(())
        };
        let b = Rc::clone(&val);
        let mut sample_callback2 = |params: &GenericMessage| -> HandlerResult<()> {
            b = 15;
            Ok(())
        };
        let mut collection = CallbackCollection::new(String::from("dummy"));
        let handler = collection.add(&mut sample_callback, AnyId).unwrap();
        let params = GenericMessage {
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
        
        let handler2 = collection
            .add(&mut sample_callback2, SomeId(SenderId(0)))
            .unwrap();
        val = 5;
        collection.call(&params);
        assert_eq!(val, 15);
        
        // Check that later-registered callbacks get run later
        let handler = collection.add(&mut sample_callback, AnyId).unwrap();
        val = 5;
        collection.call(&params);
        assert_eq!(val, 10);
        
        // This shouldn't trigger callback 2
        let params = GenericMessage {
            sender: SenderId(1),
            ..params
        };
        val = 5;
        collection.call(&params);
        assert_eq!(val, 10);
        */
    }
}
