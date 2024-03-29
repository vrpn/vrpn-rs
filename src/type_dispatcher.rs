// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::constants::GENERIC,
    data_types::{
        constants,
        id_types::*,
        message::{GenericMessage, TypedMessageBody},
        name_types::{IdWithNameAndDescription, MessageTypeName, SenderName},
        Description, MessageTypeIdentifier,
    },
    handler::*,
    name_registration::{
        ExtraDataById, InsertOrGet, IntoCorrespondingName, IterableNameRegistration,
        LocalNameRegistration, NameRegistrationContainer, PerIdData,
    },
    Result, VrpnError,
};
use bytes::Bytes;
use futures::future::LocalBoxFuture;

use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt,
    hash::Hash,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum RegisterMapping<I: UnwrappedId> {
    /// This was an existing mapping with the given ID
    Found(LocalId<I>),
    /// This was a new mapping, which has been registered and received the given ID
    NewMapping(LocalId<I>),
}

impl<I: UnwrappedId> RegisterMapping<I> {
    /// Access the wrapped ID, no matter if it was new or not.
    pub fn into_inner(&self) -> LocalId<I> {
        match self {
            RegisterMapping::Found(v) => *v,
            RegisterMapping::NewMapping(v) => *v,
        }
    }
}

impl<I: UnwrappedId> From<InsertOrGet<LocalId<I>>> for RegisterMapping<I> {
    fn from(val: InsertOrGet<LocalId<I>>) -> Self {
        match val {
            InsertOrGet::Found(i) => RegisterMapping::Found(i),
            InsertOrGet::New(i) => RegisterMapping::NewMapping(i),
        }
    }
}

type HandlerHandleInnerType = IdType;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct HandlerHandleInner(HandlerHandleInnerType);

impl HandlerHandleInner {
    fn into_handler_handle(
        self,
        message_type_filter: Option<LocalId<MessageTypeId>>,
    ) -> HandlerHandle {
        HandlerHandle(message_type_filter, self.0)
    }
}

/// A way to refer uniquely to a single added handler in a TypeDispatcher, in case
/// you want to remove it in the future.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HandlerHandle(Option<LocalId<MessageTypeId>>, HandlerHandleInnerType);

/// Type storing a boxed callback function, an optional sender ID filter,
/// and the unique-per-CallbackCollection handle that can be used to unregister a handler.
struct MsgCallbackEntry {
    handle: HandlerHandleInner,
    pub handler: Box<dyn Handler + Send>,
    pub sender_filter: Option<LocalId<SenderId>>,
}

impl fmt::Debug for MsgCallbackEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MsgCallbackEntry")
            .field("handle", &self.handle)
            .field("sender_filter", &self.sender_filter)
            .finish()
    }
}

impl MsgCallbackEntry {
    pub fn new(
        handle: HandlerHandleInner,
        handler: Box<dyn Handler + Send>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> MsgCallbackEntry {
        MsgCallbackEntry {
            handle,
            handler,
            sender_filter,
        }
    }

    /// Invokes the callback with the given msg, if the sender filter (if not None) matches.
    pub fn call(&mut self, msg: &GenericMessage) -> Result<HandlerCode> {
        if id_filter_matches(self.sender_filter, LocalId(msg.header.sender)) {
            self.handler.handle(msg)
        } else {
            Ok(HandlerCode::ContinueProcessing)
        }
    }
}

/// Stores a collection of callbacks with a name, associated with either a message type,
/// or as a "global" handler mapping called for all message types.
#[derive(Debug)]
struct CallbackCollection {
    name: Bytes,
    callbacks: Vec<Option<MsgCallbackEntry>>,
    next_handle: HandlerHandleInnerType,
}
impl Default for CallbackCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackCollection {
    /// Create CallbackCollection instance
    pub fn new() -> CallbackCollection {
        CallbackCollection {
            name: Bytes::new(),
            callbacks: Vec::new(),
            next_handle: 0,
        }
    }

    /// Add a callback with optional sender ID filter
    fn add(
        &mut self,
        handler: Box<dyn Handler + Send>,
        sender: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandleInner> {
        if self.callbacks.len() > MAX_VEC_USIZE {
            return Err(VrpnError::TooManyHandlers);
        }
        let handle = HandlerHandleInner(self.next_handle);
        self.callbacks
            .push(Some(MsgCallbackEntry::new(handle, handler, sender)));
        self.next_handle += 1;
        Ok(handle)
    }

    /// Remove a callback
    fn remove(&mut self, handle: HandlerHandleInner) -> Result<()> {
        let index = self
            .callbacks
            .iter()
            .position(|x| {
                x.as_ref()
                    .map(|handler| handler.handle == handle)
                    .unwrap_or(false)
            })
            .ok_or(VrpnError::HandlerNotFound)?;
        self.callbacks.remove(index);
        Ok(())
    }

    /// Call all callbacks (subject to sender filters) and remove the callbacks who ask for it.
    fn call(&mut self, msg: &GenericMessage) -> Result<()> {
        for entry in &mut self.callbacks.iter_mut() {
            if let Some(unwrapped_entry) = entry {
                if unwrapped_entry.call(msg)? == HandlerCode::RemoveThisHandler {
                    entry.take();
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) struct MessageTypeIndex(usize);

pub(crate) trait TryIntoIndex {
    type Index;
    fn try_into_index(self, len: usize) -> Result<Self::Index>;
}

impl TryIntoIndex for MessageTypeId {
    type Index = MessageTypeIndex;
    fn try_into_index(self, len: usize) -> Result<Self::Index> {
        use CategorizedId::*;
        match categorize_id(self, len) {
            BelowZero(v) => Err(VrpnError::InvalidId(v)),
            AboveArray(v) => Err(VrpnError::InvalidId(v)),
            InArray(index) => Ok(MessageTypeIndex(index as usize)),
        }
    }
}
#[deprecated = "Use TryIntoIndex instead"]
fn message_type_into_index(message_type: MessageTypeId, len: usize) -> Result<usize> {
    use CategorizedId::*;
    match categorize_id(message_type, len) {
        BelowZero(v) => Err(VrpnError::InvalidId(v)),
        AboveArray(v) => Err(VrpnError::InvalidId(v)),
        InArray(v) => Ok(v as usize),
    }
}

// impl TryIntoIndex for SenderId {
//     fn try_into_index(self, len: usize) -> Result<Index> {
//         use RangedId::*;
//         match determine_id_range(self, len) {
//             BelowZero(v) => Err(VrpnError::InvalidId(v)),
//             AboveArray(v) => Err(VrpnError::InvalidId(v)),
//             InArray(index) => Ok(Index(index as usize)),
//         }
//     }
// }
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Name(Bytes);

pub trait TryIntoDescriptionMessage {
    fn try_into_description_message<N: Into<Bytes>>(self, name: N) -> Result<GenericMessage>;
}

impl<I: IdWithNameAndDescription> TryIntoDescriptionMessage for I {
    fn try_into_description_message<N: Into<Bytes>>(self, name: N) -> Result<GenericMessage> {
        let desc = Description::from_id_and_name(self, name.into());
        let desc_msg = crate::data_types::TypedMessage::from(desc);
        Ok(GenericMessage::try_from(desc_msg)?)
    }
}

// Forward calls on the LocalId wrapper
impl<I: TryIntoDescriptionMessage + UnwrappedId> TryIntoDescriptionMessage for LocalId<I> {
    fn try_into_description_message<N: Into<Bytes>>(self, name: N) -> Result<GenericMessage> {
        self.into_id().try_into_description_message(name)
    }
}

pub(crate) fn try_register_system_senders_and_messages(
    sender_registration: &mut impl LocalNameRegistration<IdType = SenderId>,
    message_type_registration: &mut impl LocalNameRegistration<IdType = MessageTypeId>,
) -> Result<()> {
    sender_registration.try_insert_or_get(constants::CONTROL)?;
    message_type_registration.try_insert_or_get(constants::GOT_FIRST_CONNECTION)?;
    message_type_registration.try_insert_or_get(constants::GOT_CONNECTION)?;
    message_type_registration.try_insert_or_get(constants::DROPPED_CONNECTION)?;
    message_type_registration.try_insert_or_get(constants::DROPPED_LAST_CONNECTION)?;
    Ok(())
}

/// Structure holding and dispatching generic and message-filtered callbacks.
///
/// Unlike in the mainline C++ code, this does **not** handle "system" message types.
/// The main reason is that they are easiest hard-coded and need to access the endpoint
/// they're operating on, which can be a struggle to get past the borrow checker.
/// Thus, a hard-coded setup simply turns system messages into SystemCommand enum values,
/// which get queued through the Endpoint trait using interior mutability (e.g. with something like mpsc)
#[derive(Debug)]
pub struct TypeDispatcher {
    /// Index is the local type ID
    message_types: PerIdData<NameRegistrationContainer<MessageTypeId>, CallbackCollection>,
    generic_callbacks: CallbackCollection,
    /// Index is the local sender ID
    senders: NameRegistrationContainer<SenderId>,
}

impl Default for TypeDispatcher {
    fn default() -> TypeDispatcher {
        TypeDispatcher::new()
    }
}

impl TypeDispatcher {
    pub fn new() -> TypeDispatcher {
        let mut disp = TypeDispatcher {
            message_types: PerIdData::new(NameRegistrationContainer::default()),
            generic_callbacks: CallbackCollection::new(/* Bytes::from_static(GENERIC) */),
            senders: NameRegistrationContainer::default(),
        };

        try_register_system_senders_and_messages(&mut disp.senders, &mut disp.message_types);
        disp
    }

    /// Get a mutable borrow of the CallbackCollection associated with the supplied MessageTypeId
    /// (or the generic callbacks for None)
    fn get_type_callbacks_mut(
        &'_ mut self,
        type_id_filter: Option<LocalId<MessageTypeId>>,
    ) -> Result<&'_ mut CallbackCollection> {
        match type_id_filter {
            Some(id) => self.message_types.try_get_data_mut(id.into_id()),
            None => Ok(&mut self.generic_callbacks),
        }
    }

    /// Returns the ID for the type name, if found.
    pub fn get_type_id<T>(&self, name: T) -> Option<LocalId<MessageTypeId>>
    where
        T: Into<MessageTypeName>,
    {
        let name: MessageTypeName = name.into();
        self.message_types.try_get_id_by_name(name)
    }

    /// Calls add_type if get_type_id() returns None.
    /// Returns the corresponding MessageTypeId in all cases.
    pub fn register_type(
        &mut self,
        name: impl Into<MessageTypeName>,
    ) -> Result<RegisterMapping<MessageTypeId>> {
        Ok(self.message_types.try_insert_or_get(name)?.into())
    }

    /// Calls add_sender if get_sender_id() returns None.
    pub fn register_sender(
        &mut self,
        name: impl Into<SenderName>,
    ) -> Result<RegisterMapping<SenderId>> {
        Ok(self.senders.try_insert_or_get(name)?.into())
    }

    /// Returns the ID for the sender name, if found.
    pub fn get_sender_id(&self, name: impl Into<SenderName>) -> Option<LocalId<SenderId>> {
        self.senders.try_get_id_by_name(name)
    }

    pub fn add_handler(
        &mut self,
        handler: Box<dyn Handler + Send>,
        message_type_filter: Option<LocalId<MessageTypeId>>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandle> {
        // let mut collection = match message_type_filter {
        //     Some(message_type) => self
        //         .message_types
        //         .try_get_data_mut(message_type.into_id())?,
        //     None => &mut self.generic_callbacks,
        // };
        // collection
        self.get_type_callbacks_mut(message_type_filter)?
            .add(handler, sender_filter)
            .map(|h| h.into_handler_handle(message_type_filter))
    }

    pub fn add_typed_handler<T: 'static>(
        &mut self,
        handler: Box<T>,
        sender_filter: Option<LocalId<SenderId>>,
    ) -> Result<HandlerHandle>
    where
        T: TypedHandler + Handler + Sized,
    {
        let message_type = match T::Item::MESSAGE_IDENTIFIER {
            MessageTypeIdentifier::UserMessageName(name) => self.register_type(name)?.into_inner(),
            MessageTypeIdentifier::SystemMessageId(id) => LocalId(id),
        };
        self.add_handler(handler, Some(message_type), sender_filter)
    }

    pub fn remove_handler(&mut self, handler_handle: HandlerHandle) -> Result<()> {
        let HandlerHandle(message_type, inner) = handler_handle;
        self.get_type_callbacks_mut(message_type)?
            .remove(HandlerHandleInner(inner))
    }

    /// Akin to vrpn_TypeDispatcher::doCallbacksFor
    pub fn call(&mut self, msg: &GenericMessage) -> Result<()> {
        self.generic_callbacks.call(msg)?;
        if let Ok(mapping) = self.message_types.try_get_data_mut(msg.header.message_type) {
            mapping.call(msg)?;
        }
        Ok(())
    }

    /// caution: expensive
    fn senders_iter(&'_ self) -> impl Iterator<Item = (LocalId<SenderId>, SenderName)> + '_ {
        self.senders
            .iter()
            .map(|(id, name)| (id, SenderName(name.as_ref().clone())))
    }

    /// caution: expensive
    fn types_iter(
        &'_ self,
    ) -> impl Iterator<Item = (LocalId<MessageTypeId>, MessageTypeName)> + '_ {
        self.message_types
            .as_ref()
            .iter()
            .map(|(id, name)| (id, MessageTypeName(name.as_ref().clone())))
    }

    /// Pack all sender and type descriptions into a vector of generic messages.
    pub fn pack_all_descriptions(&self) -> Result<impl Iterator<Item = GenericMessage>> {
        let sender_messages = self
            .senders_iter()
            .map(|(id, name)| id.try_into_description_message(name.clone()))
            .collect::<Result<Vec<GenericMessage>>>()?;

        let type_messages = self
            .types_iter()
            .map(|(id, name)| id.try_into_description_message(name))
            .collect::<Result<Vec<GenericMessage>>>()?;

        Ok(sender_messages.into_iter().chain(type_messages.into_iter()))
    }
}
#[cfg(test)]
mod tests {
    use crate::data_types::{
        message::{GenericBody, GenericMessage, Message},
        MessageHeader, TimeVal,
    };
    use crate::type_dispatcher::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct SetTo10 {
        val: Arc<Mutex<i8>>,
    }
    impl Handler for SetTo10 {
        fn handle(&mut self, _msg: &GenericMessage) -> Result<HandlerCode> {
            let mut val = self.val.lock()?;
            *val = 10;
            Ok(HandlerCode::ContinueProcessing)
        }
    }
    #[derive(Debug, Clone)]
    struct SetTo15 {
        val: Arc<Mutex<i8>>,
    }
    impl Handler for SetTo15 {
        fn handle(&mut self, _msg: &GenericMessage) -> Result<HandlerCode> {
            let mut val = self.val.lock()?;
            *val = 15;
            Ok(HandlerCode::ContinueProcessing)
        }
    }
    #[test]
    fn callback_collection() {
        let val: Arc<Mutex<i8>> = Arc::new(Mutex::new(5));
        let a = Arc::clone(&val);
        let sample_callback = SetTo10 { val: a };
        let b = Arc::clone(&val);
        let sample_callback2 = SetTo15 { val: b };

        let mut collection = CallbackCollection::new();
        let handler = collection
            .add(Box::new(sample_callback.clone()), None)
            .unwrap();
        let msg = GenericMessage::from_header_and_body(
            MessageHeader::new(
                Some(TimeVal::get_time_of_day()),
                MessageTypeId(0),
                SenderId(0),
            ),
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
            .add(Box::new(sample_callback2), Some(LocalId(SenderId(0))))
            .unwrap();
        *val.lock().unwrap() = 5;
        collection.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 15);

        // Check that later-registered callbacks get run later
        let _ = collection.add(Box::new(sample_callback), None).unwrap();
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
            .add_handler(Box::new(sample_callback.clone()), None, None)
            .unwrap();
        let msg = GenericMessage::from_header_and_body(
            MessageHeader::new(
                Some(TimeVal::get_time_of_day()),
                MessageTypeId(0),
                SenderId(0),
            ),
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
            .add_handler(Box::new(sample_callback2), None, Some(LocalId(SenderId(0))))
            .unwrap();
        *val.lock().unwrap() = 5;
        dispatcher.call(&msg).unwrap();
        assert_eq!(*val.lock().unwrap(), 15);

        // Check that later-registered callbacks get run later
        let _ = dispatcher
            .add_handler(Box::new(sample_callback), None, None)
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
