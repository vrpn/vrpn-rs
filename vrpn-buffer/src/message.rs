// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, BufMut, Bytes, BytesMut};
use crate::{
    length_prefixed::{self, LengthBehavior, NullTermination},
    traits::{
        buffer::{self, Buffer, BytesMutExtras},
        unbuffer::{self, OutputResultExtras, Unbuffer},
        BufferSize, WrappedConstantSize,
    },
};
use std::{
    mem::size_of,
    net::{IpAddr, SocketAddr},
    ops::{Deref, DerefMut},
};
use vrpn_base::{
    constants::ALIGN, BaseTypeSafeId, BytesRequired, EmptyResult, Error, GenericBody,
    GenericMessage, IdType, InnerDescription, LocalId, Message, MessageBody, RemoteId, Result,
    SenderId, SequenceNumber, SequencedGenericMessage, SequencedMessage, TimeVal, TypeId,
    TypeSafeId, TypedMessageBody, UdpDescription, UdpInnerDescription,
};

impl WrappedConstantSize for SequenceNumber {
    type WrappedType = u32;
    fn get(&self) -> Self::WrappedType {
        self.0
    }
    fn new(v: Self::WrappedType) -> Self {
        SequenceNumber(v)
    }
}

impl WrappedConstantSize for SenderId {
    type WrappedType = IdType;
    fn get(&self) -> Self::WrappedType {
        TypeSafeId::get(self)
    }
    fn new(v: Self::WrappedType) -> Self {
        SenderId(v)
    }
}

impl WrappedConstantSize for TypeId {
    type WrappedType = IdType;
    fn get(&self) -> Self::WrappedType {
        TypeSafeId::get(self)
    }
    fn new(v: Self::WrappedType) -> Self {
        TypeId(v)
    }
}
// impl<T: BaseTypeSafeId> WrappedConstantSize for RemoteId<T> {
//     type WrappedType = IdType;
//     fn get(&self) -> Self::WrappedType {
//         TypeSafeId::get(self)
//     }
//     fn new(v: Self::WrappedType) -> Self {
//         RemoteId(T::new(v))
//     }
// }

#[inline]
fn compute_padding(len: usize) -> usize {
    ALIGN - (len % ALIGN)
}

#[inline]
fn padded(len: usize) -> usize {
    len + compute_padding(len)
}

/// Simple struct for wrapping all calculations related to Message<T> size.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MessageSize {
    // The unpadded size of a message body only
    pub unpadded_body_size: usize,
}

pub type LengthField = u32;

impl MessageSize {
    const UNPADDED_HEADER_SIZE: usize = 5 * 4;

    /// Get a MessageSize from the unpadded size of a message body only.
    #[inline]
    pub fn from_unpadded_body_size(unpadded_body_size: usize) -> MessageSize {
        MessageSize { unpadded_body_size }
    }

    /// Get a MessageSize from the total unpadded size of a message (header plus body)
    #[inline]
    pub fn from_unpadded_message_size(unpadded_message_size: usize) -> MessageSize {
        MessageSize::from_unpadded_body_size(
            unpadded_message_size - MessageSize::UNPADDED_HEADER_SIZE,
        )
    }
    /// Get a MessageSize from the length field of a message (padded header plus unpadded body)
    #[inline]
    pub fn from_length_field(length_field: LengthField) -> MessageSize {
        MessageSize::from_unpadded_body_size(
            length_field as usize - padded(MessageSize::UNPADDED_HEADER_SIZE),
        )
    }

    /// The unpadded size of just the message body.
    #[inline]
    pub fn unpadded_body_size(&self) -> usize {
        self.unpadded_body_size
    }

    /// The padded header size plus the unpadded body size.
    ///
    /// This is the value put in the message header's length field.
    #[inline]
    pub fn length_field(&self) -> LengthField {
        (self.unpadded_body_size + padded(MessageSize::UNPADDED_HEADER_SIZE)) as LengthField
    }

    /// The size of the body plus padding (multiple of ALIGN)
    #[inline]
    pub fn padded_body_size(&self) -> usize {
        padded(self.unpadded_body_size)
    }

    /// The number of padding bytes required to follow the message body.
    #[inline]
    pub fn body_padding(&self) -> usize {
        compute_padding(self.unpadded_body_size)
    }

    /// The total padded size of a message (header plus body, padding applied individually).
    ///
    /// This is the size of buffer actually required for this message.
    #[inline]
    pub fn padded_message_size(&self) -> usize {
        self.padded_body_size() + padded(MessageSize::UNPADDED_HEADER_SIZE)
    }
}

/// Wraps a type implementing BufMut to also track the initial remaining length,
/// to allow for automatic padding of fields.
// struct BufMutWrapper<T: BufMut>(T, usize);
// impl<T: BufMut> BufMutWrapper<T> {
//     fn new(buf: T) -> BufMutWrapper<T> {
//         let remaining = buf.remaining_mut();
//         BufMutWrapper(buf, remaining)
//     }
//     fn buffered(&self) -> usize {
//         self.1 - self.0.remaining_mut()
//     }
//     fn pad_to_align(&mut self) {
//         for _ in 0..compute_padding(self.buffered()) {
//             self.0.put_u8(0)
//         }
//     }
//     #[inline]
//     fn borrow_buf(&self) -> &T {
//         &self.0
//     }
//     #[inline]
//     fn borrow_buf_mut(&mut self) -> &mut T {
//         &mut self.0
//     }
// }

// impl<T: BufMut> Deref for BufMutWrapper<T> {
//     type Target = T;
//     #[inline]
//     fn deref(&self) -> &T {
//         self.borrow_buf()
//     }
// }

// impl<T: BufMut> DerefMut for BufMutWrapper<T> {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut T {
//         self.borrow_buf_mut()
//     }
// }

// Header is 5 i32s (padded to vrpn_ALIGN):
// - unpadded header size + unpadded body size
// - time stamp
// - sender
// - type
//
// The four bytes of "padding" are actually the sequence number,
// which are not "officially" part of the header.
//
// body is padded out to vrpn_ALIGN

fn generic_message_size(msg: &SequencedGenericMessage) -> MessageSize {
    MessageSize::from_unpadded_body_size(msg.message.body.inner.len())
}
impl BufferSize for SequencedMessage<GenericBody> {
    fn buffer_size(&self) -> usize {
        generic_message_size(self).padded_message_size()
    }
}

impl Buffer for SequencedMessage<GenericBody> {
    /// Serialize to a buffer.
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        let size = generic_message_size(self);
        if buf.remaining_mut() < size.padded_message_size() {
            return Err(Error::OutOfBuffer);
        }
        let length_field = size.length_field() as u32;

        Buffer::buffer_ref(&length_field, buf)
            .and_then(|()| self.message.header.time().buffer_ref(buf))
            .and_then(|()| self.message.header.sender().buffer_ref(buf))
            .and_then(|()| self.message.header.message_type().buffer_ref(buf))
            .and_then(|()| self.sequence_number.buffer_ref(buf))?;

        buf.put(&self.message.body.inner);
        for _ in 0..size.body_padding() {
            buf.put_u8(0);
        }
        Ok(())
    }
}

impl Unbuffer for SequencedMessage<GenericBody> {
    /// Deserialize from a buffer.
    fn unbuffer_ref(buf: &mut Bytes) -> Result<SequencedMessage<GenericBody>> {
        let initial_remaining = buf.len();
        let length_field = u32::unbuffer_ref(buf).map_exactly_err_to_at_least()?;
        let size = MessageSize::from_length_field(length_field);

        // Subtracting the length of the u32 we already unbuffered.
        let expected_remaining_bytes = size.padded_message_size() - size_of::<u32>();

        if buf.len() < expected_remaining_bytes {
            return Err(Error::NeedMoreData(BytesRequired::Exactly(
                expected_remaining_bytes - buf.len(),
            )));
        }
        let time = TimeVal::unbuffer_ref(buf)?;
        let sender = SenderId::unbuffer_ref(buf)?;
        let message_type = TypeId::unbuffer_ref(buf)?;
        let sequence_number = SequenceNumber::unbuffer_ref(buf)?;

        // Assert that handling the sequence number meant we're now aligned again.
        assert_eq!((initial_remaining - buf.len()) % ALIGN, 0);

        let body;
        {
            let mut body_buf = buf.split_to(size.unpadded_body_size());
            body = GenericBody::unbuffer_ref(&mut body_buf).map_exactly_err_to_at_least()?;
            assert_eq!(body_buf.len(), 0);
        }

        // drop padding bytes
        let _ = buf.split_to(size.body_padding());
        Ok(SequencedMessage::new(
            Some(time),
            message_type,
            sender,
            body,
            sequence_number,
        ))
    }
}

impl Unbuffer for GenericBody {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<GenericBody> {
        let my_buf = buf.clone();
        buf.advance(my_buf.len());
        Ok(GenericBody::new(my_buf))
    }
}

impl BufferSize for GenericBody {
    fn buffer_size(&self) -> usize {
        self.inner.len()
    }
}
impl Buffer for GenericBody {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        if buf.remaining_mut() < self.inner.len() {
            return Err(Error::OutOfBuffer);
        }
        buf.put(self.inner.clone());
        Ok(())
    }
}

impl<T: BaseTypeSafeId> BufferSize for InnerDescription<T> {
    fn buffer_size(&self) -> usize {
        length_prefixed::buffer_size(self.name().as_ref(), NullTermination::AddTrailingNull)
    }
}

impl<U: BaseTypeSafeId> Buffer for InnerDescription<U> {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        length_prefixed::buffer_string(
            self.name().as_ref(),
            buf,
            NullTermination::AddTrailingNull,
            LengthBehavior::IncludeNull,
        )
    }
}

impl<T: BaseTypeSafeId> Unbuffer for InnerDescription<T> {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<InnerDescription<T>> {
        length_prefixed::unbuffer_string(buf).map(InnerDescription::new)
    }
}

impl Unbuffer for UdpInnerDescription {
    fn unbuffer_ref(buf: &mut Bytes) -> Result<UdpInnerDescription> {
        let ip_buf: Vec<u8> = buf.iter().take_while(|b| **b != 0).cloned().collect();
        let ip_str = String::from_utf8_lossy(&ip_buf);
        let addr: IpAddr = ip_str
            .parse()
            .map_err(|e| Error::OtherMessage(format!("ip address parse error: {}", e)))?;
        buf.advance(ip_buf.len());

        Ok(UdpInnerDescription::new(addr))
    }
}

impl BufferSize for UdpInnerDescription {
    fn buffer_size(&self) -> usize {
        self.address.to_string().len() + 1
    }
}
impl Buffer for UdpInnerDescription {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> EmptyResult {
        let addr_str = self.address.to_string();
        if buf.remaining_mut() < (addr_str.len() + 1) {
            return Err(Error::OutOfBuffer);
        }
        buf.put(addr_str);
        buf.put_u8(0);
        Ok(())
    }
}

pub fn unbuffer_typed_message_body<T: Unbuffer + TypedMessageBody>(
    msg: GenericMessage,
) -> Result<Message<T>> {
    let mut buf = msg.body.inner.clone();
    let body =
        T::unbuffer_ref(&mut buf).map_need_more_err_to_generic_parse_err("parsing message body")?;
    if buf.len() > 0 {
        return Err(Error::OtherMessage(format!(
            "message body length was indicated as {}, but {} bytes remain unconsumed",
            msg.body.inner.len(),
            buf.len()
        )));
    }
    Ok(Message::from_header_and_body(msg.header, body))
}

pub fn make_message_body_generic<T: Buffer + TypedMessageBody>(
    msg: Message<T>,
) -> std::result::Result<GenericMessage, Error> {
    let old_body = msg.body;
    let header = msg.header;
    BytesMut::new()
        .allocate_and_buffer(old_body)
        .and_then(|body| {
            Ok(GenericMessage::from_header_and_body(
                header,
                GenericBody::new(body.freeze()),
            ))
        })
}

pub fn make_sequenced_message_body_generic<T: Buffer + TypedMessageBody>(
    msg: SequencedMessage<T>,
) -> std::result::Result<SequencedGenericMessage, Error> {
    let seq = msg.sequence_number;
    make_message_body_generic(msg.message)
        .map(|generic_msg| generic_msg.into_sequenced_message(seq))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::ConstantBufferSize;
    #[test]
    fn constant() {
        // The size field is a u32.
        let len_size = size_of::<u32>();
        let computed_size = len_size
            + TimeVal::constant_buffer_size()
            + SenderId::constant_buffer_size()
            + TypeId::constant_buffer_size();
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
    }
}
