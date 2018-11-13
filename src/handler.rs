// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{GenericMessage, Message, Result, TypedMessageBody, Unbuffer};
use std::fmt;

/// A trait implemented by structs that can handle generic messages
pub trait Handler: fmt::Debug {
    fn handle(&mut self, msg: &GenericMessage) -> Result<()>;
}

// pub trait IntoBoxedHandler {
//     fn into_boxed_handler(self) -> Box<dyn Handler>;
// }

/// A trait implemented by structs that can handle typed messages.
///
/// A blanket impl for Handler exists for all types implementing this trait,
/// so they can be treated the same. However, you probably want to use
/// add_typed_handler() instead of add_handler() so you don't need to
/// pointlessly look up/register the message type yourself.
pub trait TypedHandler: fmt::Debug {
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
