// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Bytes;
use crate::handler::*;
use crate::types::*;
use crate::{
    constants, types, Error, GenericMessage, MessageTypeIdentifier, Result, TypedMessageBody,
};

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

type HandlerHandleInnerType = types::IdType;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct HandlerHandleInner(HandlerHandleInnerType);

impl HandlerHandleInner {
    fn into_handler_handle(self, message_type: IdToHandle<TypeId>) -> HandlerHandle {
        HandlerHandle(message_type, self.0)
    }
}

/// A way to refer uniquely to a single added handler in a TypeDispatcher, in case
/// you want to remove it in the future.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HandlerHandle(IdToHandle<TypeId>, HandlerHandleInnerType);

/// Type storing a boxed callback function, an optional sender ID filter,
/// and the unique-per-CallbackCollection handle that can be used to unregister a handler.
#[derive(Debug)]
struct MsgCallbackEntry {
    handle: HandlerHandleInner,
    pub handler: Box<dyn Handler>,
    pub sender: IdToHandle<SenderId>,
}

impl MsgCallbackEntry {
    pub fn new(
        handle: HandlerHandleInner,
        handler: Box<dyn Handler>,
        sender: IdToHandle<SenderId>,
    ) -> MsgCallbackEntry {
        MsgCallbackEntry {
            handle,
            handler,
            sender,
        }
    }

    /// Invokes the callback with the given msg, if the sender filter (if not None) matches.
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
    next_handle: HandlerHandleInnerType,
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
    ) -> Result<HandlerHandleInner> {
        if self.callbacks.len() > types::MAX_VEC_USIZE {
            return Err(Error::TooManyHandlers);
        }
        let handle = HandlerHandleInner(self.next_handle);
        self.callbacks
            .push(MsgCallbackEntry::new(handle, handler, sender));
        self.next_handle += 1;
        Ok(handle)
    }

    /// Remove a callback
    fn remove(&mut self, handle: HandlerHandleInner) -> Result<()> {
        match self.callbacks.iter().position(|ref x| x.handle == handle) {
            Some(i) => {
                self.callbacks.remove(i);
                Ok(())
            }
            None => Err(Error::HandlerNotFound),
        }
    }

    /// Call all callbacks (subject to sender filters)
    fn call(&mut self, msg: &GenericMessage) -> Result<()> {
        for entry in self.callbacks.iter_mut() {
            entry.call(msg)?;
        }
        Ok(())
    }
}

fn message_type_into_index(message_type: TypeId, len: usize) -> Result<usize> {
    let raw_message_type = message_type.get();
    if raw_message_type < 0 {
        Err(Error::InvalidId(raw_message_type))?;
    }
    let index = raw_message_type as usize;
    if index >= len {
        Err(Error::InvalidId(raw_message_type))?;
    }

    Ok(index)
}

/// Structure holding and dispatching generic and message-filtered callbacks.
///
/// Unlike in the mainline C++ code, this does **not** handle "system" message types.
/// The main reason is that they are easiest hard-coded and need to access the endpoint
/// they're operating on, which can be a struggle to get past the borrow checker.
/// Thus, a hard-coded setup simply turns system messages into SystemMessage enum values,
/// which get queued through the Endpoint trait using interior mutability (e.g. with something like mpsc)type_dispatcher
#[derive(Debug)]
pub struct TypeDispatcher {
    types: Vec<CallbackCollection>,
    generic_callbacks: CallbackCollection,
    senders: Vec<SenderName>,
}

impl Default for TypeDispatcher {
    fn default() -> TypeDispatcher {
        TypeDispatcher::new()
    }
}

impl TypeDispatcher {
    pub fn new() -> TypeDispatcher {
        let mut disp = TypeDispatcher {
            types: Vec::new(),
            generic_callbacks: CallbackCollection::new(Bytes::from_static(constants::GENERIC)),
            senders: Vec::new(),
        };

        disp.register_sender(constants::CONTROL)
            .expect("couldn't register CONTROL sender");
        disp.register_type(constants::GOT_FIRST_CONNECTION)
            .expect("couldn't register GOT_FIRST_CONNECTION type");
        disp.register_type(constants::GOT_CONNECTION)
            .expect("couldn't register GOT_FIRST_CONNECTION type");
        disp.register_type(constants::DROPPED_CONNECTION)
            .expect("couldn't register DROPPED_CONNECTION type");
        disp.register_type(constants::DROPPED_LAST_CONNECTION)
            .expect("couldn't register DROPPED_LAST_CONNECTION type");
        disp
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
            None => self.add_type(name).map(RegisterMapping::NewMapping),
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
            None => self.add_sender(name).map(RegisterMapping::NewMapping),
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
        message_type: IdToHandle<TypeId>,
        sender: IdToHandle<SenderId>,
    ) -> Result<HandlerHandle> {
        self.get_type_callbacks_mut(message_type)?
            .add(handler, sender)
            .map(|h| h.into_handler_handle(message_type))
    }
    pub fn add_typed_handler<T: 'static>(
        &mut self,
        handler: Box<T>,
        sender: IdToHandle<SenderId>,
    ) -> Result<HandlerHandle>
    where
        T: TypedHandler + Handler + Sized,
    {
        let message_type = match T::Item::MESSAGE_IDENTIFIER {
            MessageTypeIdentifier::UserMessageName(name) => SomeId(self.register_type(name)?.get()),
            MessageTypeIdentifier::SystemMessageId(id) => SomeId(id),
        };
        self.add_handler(handler, message_type, sender)
    }

    pub fn remove_handler(&mut self, handler_handle: HandlerHandle) -> Result<()> {
        let HandlerHandle(message_type, inner) = handler_handle;
        self.get_type_callbacks_mut(message_type)?
            .remove(HandlerHandleInner(inner))
    }

    /// Akin to vrpn_TypeDispatcher::doCallbacksFor
    pub fn call(&mut self, msg: &GenericMessage) -> Result<()> {
        let index = message_type_into_index(msg.header.message_type, self.types.len())?;
        let mapping = &mut self.types[index];

        self.generic_callbacks.call(&msg)?;
        mapping.call(&msg)
    }
}
#[cfg(test)]
mod tests {
    use crate::type_dispatcher::*;
    use crate::{GenericBody, GenericMessage, TimeVal};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct SetTo10 {
        val: Arc<Mutex<i8>>,
    }
    impl Handler for SetTo10 {
        fn handle(&mut self, _msg: &GenericMessage) -> Result<()> {
            let mut val = self.val.lock()?;
            *val = 10;
            Ok(())
        }
    }
    #[derive(Debug, Clone)]
    struct SetTo15 {
        val: Arc<Mutex<i8>>,
    }
    impl Handler for SetTo15 {
        fn handle(&mut self, _msg: &GenericMessage) -> Result<()> {
            let mut val = self.val.lock()?;
            *val = 15;
            Ok(())
        }
    }
    #[test]
    fn callback_collection() {
        let val: Arc<Mutex<i8>> = Arc::new(Mutex::new(5));
        let a = Arc::clone(&val);
        let sample_callback = SetTo10 { val: a };
        let b = Arc::clone(&val);
        let sample_callback2 = SetTo15 { val: b };

        let mut collection = CallbackCollection::new(Bytes::from_static(b"dummy"));
        let handler = collection
            .add(Box::new(sample_callback.clone()), AnyId)
            .unwrap();
        let msg = GenericMessage::new(
            Some(TimeVal::get_time_of_day()),
            TypeId(0),
            SenderId(0),
            GenericBody::default(),
        );
        collection.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);

        collection
            .remove(handler)
            .expect("Can't remove added callback");
        // No callbacks should fire now.
        *val.lock().unwrap() = 5;
        collection.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 5);

        let _ = collection
            .add(Box::new(sample_callback2), SomeId(SenderId(0)))
            .unwrap();
        *val.lock().unwrap() = 5;
        collection.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 15);

        // Check that later-registered callbacks get run later
        let _ = collection.add(Box::new(sample_callback), AnyId).unwrap();
        *val.lock().unwrap() = 5;
        collection.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);

        // This shouldn't trigger callback 2
        let mut msg2 = msg.clone();
        msg2.header.sender = SenderId(1);
        *val.lock().unwrap() = 5;
        collection.call(&msg2).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);
    }

    #[test]
    fn type_dispatcher() {
        let val: Arc<Mutex<i8>> = Arc::new(Mutex::new(5));
        let a = Arc::clone(&val);
        let sample_callback = SetTo10 { val: a };
        let b = Arc::clone(&val);
        let sample_callback2 = SetTo15 { val: b };

        let mut dispatcher = TypeDispatcher::new();
        let handler = dispatcher
            .add_handler(Box::new(sample_callback.clone()), AnyId, AnyId)
            .unwrap();
        let msg = GenericMessage::new(
            Some(TimeVal::get_time_of_day()),
            TypeId(0),
            SenderId(0),
            GenericBody::default(),
        );
        dispatcher.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);

        dispatcher
            .remove_handler(handler)
            .expect("Can't remove added callback");
        // No callbacks should fire now.
        *val.lock().unwrap() = 5;
        dispatcher.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 5);

        let _ = dispatcher
            .add_handler(Box::new(sample_callback2), AnyId, SomeId(SenderId(0)))
            .unwrap();
        *val.lock().unwrap() = 5;
        dispatcher.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 15);

        // Check that later-registered callbacks get run later
        let _ = dispatcher
            .add_handler(Box::new(sample_callback), AnyId, AnyId)
            .unwrap();
        *val.lock().unwrap() = 5;
        dispatcher.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);

        // This shouldn't trigger callback 2
        let mut msg2 = msg.clone();
        msg2.header.sender = SenderId(1);
        *val.lock().unwrap() = 5;
        dispatcher.call(&msg2).unwrap();
        assert_eq!(*val.lock().unwrap(), 10);
    }
}
