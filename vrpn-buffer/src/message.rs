// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::BufMut;
use std::mem::size_of;
use traits::{
    buffer::{self, Buffer},
    unbuffer::{self, Unbuffer},
    BufferSize, ConstantBufferSize, WrappedConstantSize,
};

use vrpn_base::{
    constants::ALIGN,
    message::Message,
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

fn unpadded_message_header_size() -> usize {
    // The size field is a u32.
    let len_size = size_of::<u32>();
    len_size
        + TimeVal::constant_buffer_size()
        + SenderId::constant_buffer_size()
        + TypeId::constant_buffer_size()
}

impl<U: BufferSize> BufferSize for Message<U> {
    fn buffer_size(&self) -> usize {
        padded(unpadded_message_header_size()) + padded(self.buffer_size())
    }
}

fn pad_to_align<T: BufMut>(buf: &mut T, n: usize) {
    for _ in 0..compute_padding(n) {
        buf.put_u8(0)
    }
}

impl<U: Buffer> Buffer for Message<U> {
    /// Serialize to a buffer.
    fn buffer<T: BufMut>(&self, buf: &mut T) -> buffer::Result {
        let unpadded_header_len = unpadded_message_header_size();
        let unpadded_body_len = self.data.buffer_size();
        let unpadded_len: u32 = (unpadded_message_header_size() + unpadded_body_len) as u32;
        if buf.remaining_mut() < padded(unpadded_body_len) + padded(unpadded_header_len) {
            return Err(buffer::Error::OutOfBuffer);
        }
        buf.put_u32_be(unpadded_len);
        self.time
            .buffer(buf)
            .and_then(|_| self.sender.buffer(buf))
            .and_then(|_| self.message_type.buffer(buf))
            .and_then(|_| self.sequence_number.unwrap().buffer(buf))?;
        pad_to_align(buf, unpadded_header_len);
        self.data.buffer(buf).and_then(|_| {
            pad_to_align(buf, unpadded_body_len);
            Ok(())
        })
    }
}
