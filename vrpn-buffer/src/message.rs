// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    length_prefixed,
    prelude::*,
    traits::{
        buffer::{self, Buffer},
        unbuffer::{self, Output, OutputResultExtras, Unbuffer},
        BufferSize, BytesRequired, ConstantBufferSize, WrappedConstantSize,
    },
};
use bytes::{BufMut, Bytes};
use std::mem::size_of;
use vrpn_base::{
    constants::ALIGN,
    message::{GenericBody, InnerDescription, Message},
    time::TimeVal,
    types::{IdType, SenderId, SequenceNumber, TypeId},
};

impl WrappedConstantSize for SequenceNumber {
    type WrappedType = u32;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        SequenceNumber(v)
    }
}

impl WrappedConstantSize for SenderId {
    type WrappedType = IdType;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        SenderId(v)
    }
}

impl WrappedConstantSize for TypeId {
    type WrappedType = IdType;
    fn get<'a>(&'a self) -> &'a Self::WrappedType {
        &self.0
    }
    fn create(v: Self::WrappedType) -> Self {
        TypeId(v)
    }
}

fn compute_padding(len: usize) -> usize {
    ALIGN - (len % ALIGN)
}

fn padded(len: usize) -> usize {
    len + compute_padding(len)
}

const UNPADDED_MESSAGE_HEADER_SIZE: usize = 5 * 4;

fn padded_message_size(unpadded_body_size: usize) -> usize {
    padded(UNPADDED_MESSAGE_HEADER_SIZE) + padded(unpadded_body_size)
}

fn unpadded_message_size(unpadded_body_size: usize) -> usize {
    UNPADDED_MESSAGE_HEADER_SIZE + unpadded_body_size
}

fn unpadded_message_header_size() -> usize {
    // The size field is a u32.
    let len_size = size_of::<u32>();
    len_size
        + TimeVal::constant_buffer_size()
        + SenderId::constant_buffer_size()
        + TypeId::constant_buffer_size()
}

fn pad_to_align<T: BufMut>(buf: &mut T, n: usize) {
    for _ in 0..compute_padding(n) {
        buf.put_u8(0)
    }
}

// Header is 5 i32s (padded to vrpn_ALIGN):
// - unpadded header size + unpadded body size
// - time stamp
// - sender
// - type
// body is padded out to vrpn_ALIGN

impl<U: BufferSize> BufferSize for Message<U> {
    fn buffer_size(&self) -> usize {
        padded(unpadded_message_header_size()) + padded(self.data.buffer_size())
    }
}

impl<U: Buffer> Buffer for Message<U> {
    /// Serialize to a buffer.
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        let unpadded_header_len = unpadded_message_header_size();
        let unpadded_body_len = self.data.buffer_size();
        if buf.remaining_mut() < padded(unpadded_body_len) + padded(unpadded_header_len) {
            return Err(buffer::Error::OutOfBuffer);
        }
        let unpadded_len: u32 = unpadded_message_size(unpadded_body_len) as u32;

        Buffer::buffer_ref(&unpadded_len, buf)
            .and_then(|()| self.time.buffer_ref(buf))
            .and_then(|_| self.sender.buffer_ref(buf))
            .and_then(|_| self.message_type.buffer_ref(buf))
            .and_then(|_| self.sequence_number.unwrap().buffer_ref(buf))?;
        pad_to_align(buf, unpadded_header_len);
        self.data.buffer_ref(buf).and_then(|_| {
            pad_to_align(buf, unpadded_body_len);
            Ok(())
        })
    }
}

impl<U: Unbuffer> Unbuffer for Message<U> {
    /// Deserialize from a buffer.
    fn unbuffer_ref(buf: &mut Bytes) -> unbuffer::Result<Output<Message<U>>> {
        let unpadded_len = u32::unbuffer_ref(buf).map_exactly_err_to_at_least()?.data();
        let unpadded_len = unpadded_len as usize;
        let unpadded_body_len = unpadded_len - UNPADDED_MESSAGE_HEADER_SIZE;

        // Subtracting the length of the u32 we already unbuffered.
        let expected_remaining_bytes = padded_message_size(unpadded_body_len) - size_of::<u32>();

        if buf.len() < expected_remaining_bytes {
            return Err(unbuffer::Error::NeedMoreData(BytesRequired::Exactly(
                expected_remaining_bytes - buf.len(),
            )));
        }
        let time = Unbuffer::unbuffer_ref(buf)?.data();
        let sender = Unbuffer::unbuffer_ref(buf)?.data();
        let message_type = Unbuffer::unbuffer_ref(buf)?.data();
        let sequence_number = Unbuffer::unbuffer_ref(buf)?.data();

        // drop padding bytes
        buf.split_to(compute_padding(UNPADDED_MESSAGE_HEADER_SIZE));

        let data;
        {
            let mut data_buf = buf.split_to(unpadded_body_len);
            data = Unbuffer::unbuffer_ref(&mut data_buf)
                .map_exactly_err_to_at_least()?
                .data();
            if data_buf.len() > 0 {
                return Err(unbuffer::Error::ParseError(format!(
                    "message body length was indicated as {}, but {} bytes remain unconsumed",
                    unpadded_body_len,
                    data_buf.len()
                )));
            }
        }

        // drop padding bytes
        buf.split_to(compute_padding(unpadded_body_len));
        Ok(Output(Message::new(
            Some(time),
            message_type,
            sender,
            data,
            Some(sequence_number),
        )))
    }
}

impl Unbuffer for GenericBody {
    fn unbuffer_ref(buf: &mut Bytes) -> unbuffer::Result<Output<GenericBody>> {
        let my_buf = buf.clone();
        buf.advance(my_buf.len());
        Ok(Output(GenericBody::new(my_buf)))
    }
}

impl BufferSize for InnerDescription {
    fn buffer_size(&self) -> usize {
        length_prefixed::buffer_size(self.name.as_ref())
    }
}

impl Buffer for InnerDescription {
    fn buffer_ref<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        length_prefixed::buffer_string(self.name.as_ref(), buf)
    }
}

impl Unbuffer for InnerDescription {
    fn unbuffer_ref(buf: &mut Bytes) -> unbuffer::Result<Output<InnerDescription>> {
        length_prefixed::unbuffer_string(buf).map(|b| Output(InnerDescription::new(b)))
    }
}

fn unbuffer_typed_message_body<T: Unbuffer>(
    msg: Message<GenericBody>,
) -> unbuffer::Result<Message<T>> {
    let mut buf = msg.data.body_bytes.clone();
    let data = Unbuffer::unbuffer_ref(&mut buf)
        .map_need_more_err_to_generic_parse_err("parsing message body")?
        .data();
    if buf.len() > 0 {
        return Err(unbuffer::Error::ParseError(format!(
            "message body length was indicated as {}, but {} bytes remain unconsumed",
            msg.data.body_bytes.len(),
            buf.len()
        )));
    }
    Ok(Message::new(
        Some(msg.time),
        msg.message_type,
        msg.sender,
        data,
        msg.sequence_number,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant() {
        assert_eq!(UNPADDED_MESSAGE_HEADER_SIZE, unpadded_message_header_size());
    }
}
