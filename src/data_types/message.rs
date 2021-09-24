// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! Message types and message size computations.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::convert::TryFrom;

use crate::{
    buffer_unbuffer::{
        buffer::{self, BytesMutExtras},
        size_requirement::*,
        unbuffer::{self, UnbufferFrom},
        BufferSize, BufferUnbufferError, ConstantBufferSize,
    },
    Result, VrpnError,
};

use super::{id_types::*, name_types::StaticMessageTypeName, TimeVal};

/// The identification used for a typed message body type.
#[derive(Debug)]
pub enum MessageTypeIdentifier {
    /// User message types are identified by a string which is dynamically associated
    /// with an ID on each side.
    UserMessageName(StaticMessageTypeName),

    /// System message types are identified by a constant, negative message type ID.
    ///
    // TODO: find a way to assert/enforce that this is negative - maybe a SystemTypeId type?
    SystemMessageId(MessageTypeId),
}

/// Trait for typed message bodies.
pub trait TypedMessageBody: std::fmt::Debug {
    /// The name string (for user messages) or type ID (for system messages) used to identify this message type.
    const MESSAGE_IDENTIFIER: MessageTypeIdentifier;
}

/// Header information for a message.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageHeader {
    pub time: TimeVal,
    pub message_type: MessageTypeId,
    pub sender: SenderId,
}

impl MessageHeader {
    /// Constructor for a message header
    pub fn new(
        time: Option<TimeVal>,
        message_type: impl IntoId<BaseId = MessageTypeId>,
        sender: impl IntoId<BaseId = SenderId>,
    ) -> MessageHeader {
        MessageHeader {
            time: time.unwrap_or_else(TimeVal::get_time_of_day),
            message_type: message_type.into_id(),
            sender: sender.into_id(),
        }
    }
}

impl ConstantBufferSize for MessageHeader {
    fn constant_buffer_size() -> usize
    where
        Self: Sized,
    {
        TimeVal::constant_buffer_size()
            + MessageTypeId::constant_buffer_size()
            + SenderId::constant_buffer_size()
    }
}

impl unbuffer::UnbufferFrom for MessageHeader {
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        let time = TimeVal::unbuffer_from(buf)?;
        let sender = SenderId::unbuffer_from(buf)?;
        let message_type = MessageTypeId::unbuffer_from(buf)?;
        Ok(MessageHeader {
            time,
            sender,
            message_type,
        })
    }
}

impl buffer::BufferTo for MessageHeader {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, MessageHeader::constant_buffer_size())?;
        self.time.buffer_to(buf)?;
        self.sender.buffer_to(buf)?;
        self.message_type.buffer_to(buf)?;
        Ok(())
    }
}

/// Trait unifying `TypedMessage<T>` and `GenericMessage`
pub trait Message
where
    Self: Sized,
{
    type Body;
    /// Create a message by combining a header and a body.
    fn from_header_and_body(header: MessageHeader, body: Self::Body) -> Self;

    /// Access the header
    fn header_ref(&self) -> &MessageHeader;

    /// Access the body
    fn body_ref(&self) -> &Self::Body;

    /// true if the message type indicates that it is a "system" message (type ID < 0)
    fn is_system_message(&self) -> bool {
        self.header_ref().message_type.is_system_message()
    }
}

/// A message with header information, almost ready to be buffered to the wire.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TypedMessage<T: TypedMessageBody> {
    pub header: MessageHeader,
    pub body: T,
}

impl<T: TypedMessageBody> TypedMessage<T> {
    pub fn new(
        time: Option<TimeVal>,
        message_type: impl IntoId<BaseId = MessageTypeId>,
        sender: impl IntoId<BaseId = SenderId>,
        body: T,
    ) -> TypedMessage<T> {
        TypedMessage {
            header: MessageHeader::new(time, message_type, sender),
            body,
        }
    }

    /// Create a message by combining a header and a body.
    pub fn from_header_and_body(header: MessageHeader, body: T) -> TypedMessage<T> {
        TypedMessage { header, body }
    }
}
impl<T: TypedMessageBody> Message for TypedMessage<T> {
    type Body = T;

    fn from_header_and_body(header: MessageHeader, body: Self::Body) -> Self {
        Self { header, body }
    }

    fn header_ref(&self) -> &MessageHeader {
        &self.header
    }

    fn body_ref(&self) -> &Self::Body {
        &self.body
    }
}

impl<T: TypedMessageBody + unbuffer::UnbufferFrom> TryFrom<&GenericMessage> for TypedMessage<T> {
    type Error = VrpnError;

    /// Try parsing a generic message into a typed message
    ///
    /// # Errors
    /// - If the unbuffering of the given type fails
    /// - If the generic message's body isn't fully consumed by the typed message body
    fn try_from(msg: &GenericMessage) -> std::result::Result<Self, Self::Error> {
        let mut buf = msg.body.inner.clone();
        let body = T::unbuffer_from(&mut buf)
            .map_err(BufferUnbufferError::map_bytes_required_to_size_mismatch)?;
        if !buf.is_empty() {
            return Err(VrpnError::OtherMessage(format!(
                "message body length was indicated as {}, but {} bytes remain unconsumed",
                msg.body.inner.len(),
                buf.len()
            )));
        }
        Ok(TypedMessage::from_header_and_body(msg.header.clone(), body))
    }
}

impl<T: TypedMessageBody + unbuffer::UnbufferFrom> TypedMessage<T> {
    #[deprecated]
    pub fn try_from_generic(msg: &GenericMessage) -> Result<TypedMessage<T>> {
        let mut buf = msg.body.inner.clone();
        let body = T::unbuffer_from(&mut buf)
            .map_err(BufferUnbufferError::map_bytes_required_to_size_mismatch)?;
        if !buf.is_empty() {
            return Err(VrpnError::OtherMessage(format!(
                "message body length was indicated as {}, but {} bytes remain unconsumed",
                msg.body.inner.len(),
                buf.len()
            )));
        }
        Ok(TypedMessage::from_header_and_body(msg.header.clone(), body))
    }
}

/// A special type of message, with just an (exact-size) buffer as the body.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GenericMessage {
    pub header: MessageHeader,
    pub body: GenericBody,
}

impl GenericMessage {
    /// Consumes this message and returns a new SequencedMessage, which the supplied sequence number has been added to.
    pub fn into_sequenced_message(
        self,
        sequence_number: SequenceNumber,
    ) -> SequencedGenericMessage {
        SequencedGenericMessage {
            message: self,
            sequence_number,
        }
    }
}
impl Message for GenericMessage {
    type Body = GenericBody;

    fn from_header_and_body(header: MessageHeader, body: Self::Body) -> Self {
        Self { header, body }
    }

    fn header_ref(&self) -> &MessageHeader {
        &self.header
    }

    fn body_ref(&self) -> &Self::Body {
        &self.body
    }
}

impl<T: TypedMessageBody + buffer::BufferTo> TryFrom<TypedMessage<T>> for GenericMessage {
    type Error = BufferUnbufferError;

    fn try_from(value: TypedMessage<T>) -> std::result::Result<Self, Self::Error> {
        let old_body = value.body;
        let header = value.header;
        let mut buf = BytesMut::with_capacity(old_body.buffer_size());
        old_body.buffer_to(&mut buf)?;
        Ok(GenericMessage::from_header_and_body(
            header,
            GenericBody::new(buf.freeze()),
        ))
    }
}

impl<T: TypedMessageBody + unbuffer::UnbufferFrom> TryFrom<GenericMessage> for TypedMessage<T> {
    type Error = BufferUnbufferError;

    fn try_from(value: GenericMessage) -> std::result::Result<Self, Self::Error> {
        let mut buf = value.body.clone().into_inner();
        let typed_body = T::unbuffer_from(&mut buf)?;
        Ok(TypedMessage {
            header: value.header,
            body: typed_body,
        })
    }
}

/// A generic message with header information and sequence number, ready to be buffered to the wire.
///
/// Wraps `GenericMessage`
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SequencedGenericMessage {
    message: GenericMessage,
    pub sequence_number: SequenceNumber,
}

impl SequencedGenericMessage {
    /// Convert into the contained `GenericMessage`
    pub fn into_inner(self) -> GenericMessage {
        self.message
    }
    /// Access a reference to the contained `GenericMessage`
    pub fn message(&self) -> &GenericMessage {
        &self.message
    }

    /// Serialize to a buffer.
    pub fn try_into_buf(self) -> std::result::Result<Bytes, BufferUnbufferError> {
        let mut buf = BytesMut::with_capacity(self.buffer_size());

        let size = generic_message_size(&self);
        let length_field = size.length_field() as u32;

        buffer::BufferTo::buffer_to(&length_field, &mut buf)?;
        buffer::BufferTo::buffer_to(&self.message.header, &mut buf)?;
        buffer::BufferTo::buffer_to(&self.sequence_number, &mut buf)?;

        buf.put_slice(&self.message.body.inner);
        for _ in 0..size.body_padding() {
            buf.put_u8(0);
        }
        Ok(buf.freeze())
    }

    /// Deserialize from a buffer.
    pub fn try_read_from_buf<T: Buf + Clone>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        let initial_remaining = buf.remaining();
        let length_field = unbuffer::peek_u32(buf).ok_or_else(|| {
            BufferUnbufferError::from(SizeRequirement::AtLeast(u32::constant_buffer_size()))
        })?;
        let size = MessageSize::from_length_field(length_field);
        unbuffer::check_unbuffer_remaining(buf, size.padded_message_size())?;

        assert_ne!(buf.remaining(), 0);
        // now we can actually unbuffer the length since we checked to make sure we have it.
        let _ = u32::unbuffer_from(buf)?;
        let header = MessageHeader::unbuffer_from(buf)?;
        assert_ne!(buf.remaining(), 0);

        let sequence_number = SequenceNumber::unbuffer_from(buf)?;
        assert_ne!(buf.remaining(), 0);

        // Assert that handling the sequence number meant we're now aligned again.
        assert_eq!(
            (initial_remaining - buf.remaining()) % crate::buffer_unbuffer::constants::ALIGN,
            0
        );

        let body;
        {
            let mut body_buf = buf.copy_to_bytes(size.unpadded_body_size());
            body = GenericBody::unbuffer_from(&mut body_buf)
                .map_err(ExpandSizeRequirement::expand_size_requirement)?;
            assert_eq!(body_buf.remaining(), 0);
        }

        // drop padding bytes
        let _ = buf.copy_to_bytes(size.body_padding());
        Ok(SequencedGenericMessage {
            message: GenericMessage { header, body },
            sequence_number,
        })
    }
}

impl BufferSize for SequencedGenericMessage {
    fn buffer_size(&self) -> usize {
        generic_message_size(self).padded_message_size()
    }
}

/// Generic body struct used in unbuffering process, before dispatch on type to fully decode.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct GenericBody {
    inner: Bytes,
}

impl GenericBody {
    /// Create a generic body from a bytes buffer.
    pub fn new(inner: Bytes) -> GenericBody {
        GenericBody { inner }
    }

    /// Consume and return the inner Bytes
    pub fn into_inner(self) -> Bytes {
        self.inner
    }
}

#[inline]
const fn compute_padding(len: usize) -> usize {
    let remainder = len % crate::buffer_unbuffer::constants::ALIGN;
    if remainder != 0 {
        crate::buffer_unbuffer::constants::ALIGN - remainder
    } else {
        0
    }
}

#[inline]
const fn padded(len: usize) -> usize {
    len + compute_padding(len)
}

/// Simple struct for wrapping all calculations related to Message<T> size.
///
/// Header is 5 i32s (padded to `vrpn_ALIGN`):
/// - padded header size + unpadded body size
/// - time stamp
/// - sender
/// - type
///
/// The four bytes of "padding" are actually the sequence number,
/// which are not "officially" part of the header.
///
/// body is padded out to `vrpn_ALIGN`
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageSize {
    // The unpadded size of a message body only
    pub unpadded_body_size: usize,
}

/// The type of the length field in the header.
pub type LengthField = u32;

impl MessageSize {
    const UNPADDED_HEADER_SIZE: usize = 5 * 4;

    /// Get a MessageSize from the unpadded size of a message body only.
    #[inline]
    pub const fn from_unpadded_body_size(unpadded_body_size: usize) -> MessageSize {
        MessageSize { unpadded_body_size }
    }

    /// Get a MessageSize from the total unpadded size of a message (header plus body)
    #[inline]
    pub const fn from_unpadded_message_size(unpadded_message_size: usize) -> MessageSize {
        MessageSize::from_unpadded_body_size(
            unpadded_message_size - MessageSize::UNPADDED_HEADER_SIZE,
        )
    }
    /// Get a MessageSize from the length field of a message (padded header plus unpadded body)
    #[inline]
    pub const fn from_length_field(length_field: LengthField) -> MessageSize {
        MessageSize::from_unpadded_body_size(
            length_field as usize - padded(MessageSize::UNPADDED_HEADER_SIZE),
        )
    }

    /// The unpadded size of just the message body.
    #[inline]
    pub const fn unpadded_body_size(&self) -> usize {
        self.unpadded_body_size
    }

    /// The padded header size plus the unpadded body size.
    ///
    /// This is the value put in the message header's length field.
    #[inline]
    pub const fn length_field(&self) -> LengthField {
        (self.unpadded_body_size + padded(MessageSize::UNPADDED_HEADER_SIZE)) as LengthField
    }

    /// The size of the body plus padding (multiple of ALIGN)
    #[inline]
    pub const fn padded_body_size(&self) -> usize {
        padded(self.unpadded_body_size)
    }

    /// The number of padding bytes required to follow the message body.
    #[inline]
    pub const fn body_padding(&self) -> usize {
        compute_padding(self.unpadded_body_size)
    }

    /// The total padded size of a message (header plus body, padding applied individually).
    ///
    /// This is the size of buffer actually required for this message.
    #[inline]
    pub const fn padded_message_size(&self) -> usize {
        self.padded_body_size() + padded(MessageSize::UNPADDED_HEADER_SIZE)
    }
}

fn generic_message_size(msg: &SequencedGenericMessage) -> MessageSize {
    MessageSize::from_unpadded_body_size(msg.message.body.inner.len())
}

impl unbuffer::UnbufferFrom for GenericBody {
    /// This takes all the bytes!
    fn unbuffer_from<T: Buf>(buf: &mut T) -> unbuffer::UnbufferResult<Self> {
        let my_bytes = buf.copy_to_bytes(buf.remaining());
        Ok(GenericBody::new(my_bytes))
    }
}

impl BufferSize for GenericBody {
    fn buffer_size(&self) -> usize {
        self.inner.len()
    }
}
impl buffer::BufferTo for GenericBody {
    fn buffer_to<T: BufMut>(&self, buf: &mut T) -> buffer::BufferResult {
        buffer::check_buffer_remaining(buf, self.inner.len())?;
        buf.put(self.inner.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::buffer_unbuffer::{constants::ALIGN, ConstantBufferSize};

    use super::*;

    extern crate static_assertions;
    static_assertions::assert_not_impl_any!(GenericBody: TypedMessageBody);

    #[test]
    fn constant() {
        // The size field is a u32.
        let len_size = size_of::<u32>();
        let computed_size = len_size
            + TimeVal::constant_buffer_size()
            + SenderId::constant_buffer_size()
            + MessageTypeId::constant_buffer_size();
        assert_eq!(MessageSize::UNPADDED_HEADER_SIZE,
            computed_size,
            "The constant for header size should match the actual size of the fields in the header.");
        assert_eq!(
            (MessageSize::UNPADDED_HEADER_SIZE + SequenceNumber::constant_buffer_size()) % ALIGN,
            0,
            "The sequence number should make our header need no additional padding."
        );
    }

    #[test]
    fn sizes() {
        // Based on the initial "VRPN Control" sender ID message
        assert_eq!(
            MessageSize::from_unpadded_body_size(17).unpadded_body_size(),
            17
        );
        assert_eq!(
            MessageSize::from_unpadded_body_size(17).padded_body_size(),
            24
        );
        assert_eq!(MessageSize::UNPADDED_HEADER_SIZE, 20);
        assert_eq!(
            MessageSize::from_unpadded_body_size(17).padded_message_size(),
            48
        );
        assert_eq!(MessageSize::from_unpadded_body_size(17).length_field(), 41);
        assert_eq!(MessageSize::from_length_field(41).length_field(), 41);
        assert_eq!(MessageSize::from_length_field(41).unpadded_body_size(), 17);

        // Based on the second message, which is closer to the edge and thus breaks things.
        assert_eq!(
            MessageSize::from_unpadded_body_size(13),
            MessageSize::from_length_field(0x25)
        );
        assert_eq!(
            MessageSize::from_unpadded_body_size(13).padded_body_size(),
            16
        );
        assert_eq!(
            MessageSize::from_unpadded_body_size(13).length_field(),
            24 + 13
        );
        assert_eq!(MessageSize::from_length_field(37).length_field(), 37);

        assert_eq!(MessageSize::from_length_field(37).padded_message_size(), 40);
    }
    proptest! {
        #[test]
        fn length_field_matches(len in 0u32..10000) {
            let len = len as usize;
            prop_assert_eq!(
            MessageSize::from_unpadded_body_size(len).length_field(),
            transcribed_padding_function(len).len_field);
        }

        #[test]
        fn total_length_matches(len in 0u32..10000) {
            let len = len as usize;
            prop_assert_eq!(
            MessageSize::from_unpadded_body_size(len).padded_message_size(),
            transcribed_padding_function(len).total_len);
        }

        #[test]
        fn roundtrip(len in 20u32..10000)  {
            prop_assert_eq!(MessageSize::from_length_field(len).length_field(), len);
        }
    }

    struct Lengths {
        header_len: usize,
        total_len: usize,
        len_field: u32,
    }

    /// This is a relatively literal translation of the size computations in vrpn_Endpoint::marshall_message,
    /// used as ground truth in testing.
    fn transcribed_padding_function(len: usize) -> Lengths {
        let mut ceil_len = len;
        if (len % ALIGN) != 0 {
            ceil_len += ALIGN - len % ALIGN;
        }

        let mut header_len = 5 * std::mem::size_of::<i32>();
        if (header_len % ALIGN) != 0 {
            header_len += ALIGN - header_len % ALIGN;
        }
        let total_len = header_len + ceil_len;
        Lengths {
            header_len,
            total_len,
            len_field: (header_len + len) as u32,
        }
    }
}
