// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    EmptyMessage, GenericMessage, Message, MessageHeader, Result, TypedMessageBody, Unbuffer,
};
use std::fmt;

/// A trait implemented by structs that can handle generic messages
pub trait Handler {
    fn handle(&mut self, msg: &GenericMessage) -> Result<()>;
}

/// A trait implemented by structs that can handle typed messages.
///
/// A blanket impl for Handler exists for all types implementing this trait,
/// so they can be treated the same. However, you probably want to use
/// add_typed_handler() instead of add_handler() so you don't need to
/// pointlessly look up/register the message type yourself.
pub trait TypedHandler {
    type Item: TypedMessageBody + Unbuffer + fmt::Debug;
    fn handle_typed(&mut self, msg: &Message<Self::Item>) -> Result<()>;
}

impl<T> Handler for T
where
    T: TypedHandler,
{
    fn handle(&mut self, msg: &GenericMessage) -> Result<()> {
        let typed_msg: Message<T::Item> = Message::try_from_generic(msg)?;
        self.handle_typed(&typed_msg)
    }
}

/// A trait implemented by structs that can handle typed messages with no body.
///
/// A blanket impl for Handler exists for all types implementing this trait,
/// so they can be treated the same. However, you probably want to use
/// add_typed_handler() instead of add_handler() so you don't need to
/// pointlessly look up/register the message type yourself.
pub trait TypedBodylessHandler {
    type Item: TypedMessageBody + EmptyMessage + Unbuffer + fmt::Debug;
    fn handle_typed_bodyless(&mut self, header: &MessageHeader) -> Result<()>;
}

impl<T> TypedHandler for T
where
    T: TypedBodylessHandler,
{
    type Item = <Self as TypedBodylessHandler>::Item;
    fn handle_typed(&mut self, msg: &Message<Self::Item>) -> Result<()> {
        self.handle_typed_bodyless(&msg.header)
    }
}
