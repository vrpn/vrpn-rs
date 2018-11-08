// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::error::{Error, Result};
use std::fmt;
use vrpn_base::{
    constants,
    message::GenericMessage,
    types::{self, *},
};
use vrpn_buffer::{buffer, unbuffer};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum RegisterMapping<T: BaseTypeSafeId> {
    /// This was an existing mapping with the given ID
    Found(T),
    /// This was a new mapping, which has been registered and received the given ID
    NewMapping(T),
}

impl<T: BaseTypeSafeId> RegisterMapping<T> {
    /// Access the wrapped ID, no matter if it was new or not.
    pub fn get(&self) -> T {
        match self {
            RegisterMapping::Found(v) => *v,
            RegisterMapping::NewMapping(v) => *v,
        }
    }
}
type HandlerInnerType = types::IdType;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HandlerHandle(HandlerInnerType);
// pub type HandlerFnMutInner = FnMut(&GenericMessage) -> Result<()>;

pub trait Handler: fmt::Debug {
    // TODO replace with an associated const once we can do that and still box it.
    fn message_type(&self) -> IdToHandle<TypeId>;
    fn handle(&mut self, msg: &GenericMessage) -> Result<()>;
}

pub trait SystemHandler: fmt::Debug {
    // TODO replace with an associated const once we can do that and still box it.
    fn message_type(&self) -> TypeId;
    fn handle(&mut self, msg: &GenericMessage) -> Result<()>;
}

// pub struct HandlerFnMut(HandlerFnMutInner);
// impl<'a> fmt::Debug for HandlerFnMut {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "HandlerFnMut")
//     }
// }

/// Type storing a boxed callback function, an optional sender ID filter,
/// and the unique-per-CallbackCollection handle that can be used to unregister a handler.
#[derive(Debug)]
struct MsgCallbackEntry {
    handle: HandlerHandle,
    pub handler: Box<dyn Handler>,
    pub sender: IdToHandle<SenderId>,
}

// impl<'a> fmt::Debug for MsgCallbackEntry {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             f,
//             "MsgCallbackEntry {{ handle: {:?}, sender: {:?} }}",
//             self.handle, self.sender
//         )
//     }
// }
impl MsgCallbackEntry {
    pub fn new(
        handle: HandlerHandle,
        handler: Box<dyn Handler>,
        sender: IdToHandle<SenderId>,
    ) -> MsgCallbackEntry {
        MsgCallbackEntry {
            handle,
            handler,
            sender,
        }
    }

    /// Invokes the callback with the given params, if the sender filter (if not None) matches.
    pub fn call<'a>(&mut self, msg: &'a GenericMessage) -> Result<()> {
        if self.sender.matches(&msg.header.sender) {
            self.handler.handle(msg)
        } else {
            Ok(())
        }
    }
}

/// Stores a collection of callbacks with a name, associated with either a message type,
/// or as a "global" handler mapping called for all message types.
#[derive(Debug)]
struct CallbackCollection {
    name: Bytes,
    callbacks: Vec<MsgCallbackEntry>,
    next_handle: HandlerInnerType,
}

impl CallbackCollection {
    /// Create CallbackCollection instance
    pub fn new(name: Bytes) -> CallbackCollection {
        CallbackCollection {
            name,
            callbacks: Vec::new(),
            next_handle: 0,
        }
    }

    /// Add a callback with optional sender ID filter
    fn add(
        &mut self,
        handler: Box<dyn Handler>,
        sender: IdToHandle<SenderId>,
    ) -> Result<HandlerHandle> {
        if self.callbacks.len() > types::MAX_VEC_USIZE {
            return Err(Error::TooManyHandlers);
        }
        let handle = HandlerHandle(self.next_handle);
        self.callbacks
            .push(MsgCallbackEntry::new(handle, handler, sender));
        self.next_handle += 1;
        Ok(handle)
    }

    /// Remove a callback
    fn remove(&mut self, handle: HandlerHandle) -> Result<()> {
        match self.callbacks.iter().position(|ref x| x.handle == handle) {
            Some(i) => {
                self.callbacks.remove(i);
                Ok(())
            }
            None => Err(Error::HandlerNotFound),
        }
    }

    /// Call all callbacks (subject to sender filters)
    fn call(&mut self, params: &GenericMessage) -> Result<()> {
        for ref mut entry in self.callbacks.iter_mut() {
            entry.call(params)?;
        }
        Ok(())
    }
}

fn system_message_type_into_index(message_type: TypeId) -> Result<usize> {
    let raw_message_type = message_type.get();
    if raw_message_type >= 0 {
        Err(Error::InvalidTypeId(raw_message_type))?;
    }
    Ok((-raw_message_type) as usize)
}

fn message_type_into_index(message_type: TypeId, len: usize) -> Result<usize> {
    let raw_message_type = message_type.get();
    let index = raw_message_type as usize;
    if index < 0 || index >= len {
        Err(Error::InvalidTypeId(raw_message_type))?;
    }

    Ok(index)
}

#[derive(Debug)]
pub struct TypeDispatcher {
    types: Vec<CallbackCollection>,
    generic_callbacks: CallbackCollection,
    senders: Vec<SenderName>,
    system_callbacks: Vec<Option<Box<dyn SystemHandler>>>,
}

// impl<'a> fmt::Debug for TypeDispatcher {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_struct("TypeDispatcher")
//             .field("types", &self.types)
//             .field("senders", &self.senders)
//             .field("system_callbacks", &self.system_callbacks)
//             .finish()
//     }
// }

impl TypeDispatcher {
    pub fn new() -> Result<TypeDispatcher> {
        let mut disp = TypeDispatcher {
            types: Vec::new(),
            generic_callbacks: CallbackCollection::new(Bytes::from_static(constants::GENERIC)),
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
    fn get_type_callbacks_mut<'a>(
        &'a mut self,
        type_id: IdToHandle<TypeId>,
    ) -> Result<&'a mut CallbackCollection> {
        match type_id {
            SomeId(i) => {
                let index = message_type_into_index(i, self.types.len())?;
                Ok(&mut self.types[index])
            }
            AnyId => Ok(&mut self.generic_callbacks),
        }
    }

    /// Get a borrow of the CallbackCollection associated with the supplied TypeId
    /// (or the generic callbacks for AnyId)
    fn get_type_callbacks<'a>(
        &'a self,
        type_id: IdToHandle<TypeId>,
    ) -> Result<&'a CallbackCollection> {
        match type_id {
            SomeId(i) => {
                let index = message_type_into_index(i, self.types.len())?;
                Ok(&self.types[index])
            }
            AnyId => Ok(&self.generic_callbacks),
        }
    }

    pub fn add_type(&mut self, name: impl Into<TypeName>) -> Result<TypeId> {
        if self.types.len() > MAX_VEC_USIZE {
            return Err(Error::TooManyMappings);
        }
        self.types.push(CallbackCollection::new(name.into().0));
        Ok(TypeId((self.types.len() - 1) as IdType))
    }

    pub fn add_sender(&mut self, name: impl Into<SenderName>) -> Result<SenderId> {
        if self.senders.len() > (IdType::max_value() - 2) as usize {
            return Err(Error::TooManyMappings);
        }
        self.senders.push(name.into());
        Ok(SenderId((self.senders.len() - 1) as IdType))
    }

    /// Returns the ID for the type name, if found.
    pub fn get_type_id<T>(&self, name: T) -> Option<TypeId>
    where
        T: Into<TypeName>,
    {
        let name: TypeName = name.into();
        let name: Bytes = name.into();
        self.types
            .iter()
            .position(|ref x| x.name == name)
            .map(|i| TypeId(i as IdType))
    }

    /// Calls add_type if get_type_id() returns None.
    /// Returns the corresponding TypeId in all cases.
    pub fn register_type(&mut self, name: impl Into<TypeName>) -> Result<RegisterMapping<TypeId>> {
        let name: TypeName = name.into();
        match self.get_type_id(name.clone()) {
            Some(i) => Ok(RegisterMapping::Found(i)),
            None => self.add_type(name).map(|i| RegisterMapping::NewMapping(i)),
        }
    }

    /// Calls add_sender if get_sender_id() returns None.
    pub fn register_sender(
        &mut self,
        name: impl Into<SenderName>,
    ) -> Result<RegisterMapping<SenderId>> {
        let name: SenderName = name.into();
        match self.get_sender_id(name.clone()) {
            Some(i) => Ok(RegisterMapping::Found(i)),
            None => self
                .add_sender(name)
                .map(|i| RegisterMapping::NewMapping(i)),
        }
    }

    /// Returns the ID for the sender name, if found.
    pub fn get_sender_id(&self, name: impl Into<SenderName>) -> Option<SenderId> {
        let name: SenderName = name.into();
        self.senders
            .iter()
            .position(|ref x| **x == name)
            .map(|i| SenderId(i as IdType))
    }

    pub fn add_handler(
        &mut self,
        handler: Box<dyn Handler>,
        sender: IdToHandle<SenderId>,
    ) -> Result<HandlerHandle> {
        self.get_type_callbacks_mut(handler.message_type())?
            .add(handler, sender)
    }

    pub fn do_callbacks_for(&mut self, msg: GenericMessage) -> Result<()> {
        let index = message_type_into_index(msg.header.message_type, self.types.len())?;
        let ref mut mapping = &mut self.types[index];

        self.generic_callbacks.call(&msg)?;
        mapping.call(&msg)
    }

    pub fn do_system_callbacks_for(&mut self, msg: GenericMessage) -> Result<()> {
        let index = system_message_type_into_index(msg.header.message_type)?;
        if index >= self.system_callbacks.len() {
            // Not an error to try to call an unhandled system message
            return Ok(());
        }
        match self.system_callbacks[index] {
            Some(ref mut handler) => handler.handle(&msg),
            None => Ok(()),
        }
    }

    pub fn set_system_handler(&mut self, handler: Box<dyn SystemHandler>) -> Result<()> {
        let index = system_message_type_into_index(handler.message_type())?;
        while index >= self.system_callbacks.len() {
            self.system_callbacks.push(None);
        }
        self.system_callbacks[index] = Some(handler);
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
        let mut sample_callback = |params: &GenericMessage| -> Result<()> {
            a = 10;
            Ok(())
        };
        let b = Rc::clone(&val);
        let mut sample_callback2 = |params: &GenericMessage| -> Result<()> {
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
