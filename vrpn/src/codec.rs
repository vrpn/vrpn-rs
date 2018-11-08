// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    base::message::GenericMessage,
    buffer::{buffer, message::MessageSize, unbuffer, Buffer, Output, Unbuffer},
    prelude::*,
};
use bytes::BytesMut;
use pretty_hex::*;
use tokio::{
    codec::{Decoder, Encoder, Framed},
    prelude::*,
};

pub struct FramedMessageCodec;
impl Decoder for FramedMessageCodec {
    type Item = GenericMessage;
    type Error = unbuffer::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<GenericMessage>, Self::Error> {
        let initial_len = buf.len();
        let size_len = u32::constant_buffer_size();
        if initial_len < size_len {
            return Ok(None);
        }
        let combined_size = buf
            .clone()
            .split_to(u32::constant_buffer_size())
            .freeze()
            .unbuffer::<u32>()
            .map_exactly_err_to_at_least()?
            .data() as usize;
        let size = MessageSize::from_unpadded_message_size(combined_size);
        if buf.len() < size.padded_message_size() {
            return Ok(None);
        }
        println!(
            "Got {} bytes, need {} for this message",
            buf.len(),
            size.padded_message_size()
        );
        let taken_buf = buf.split_to(size.padded_message_size());
        let mut temp_buf = taken_buf.clone().freeze();
        println!("{:?}", temp_buf.as_ref().hex_dump());
        match GenericMessage::unbuffer_ref(&mut temp_buf) {
            Ok(Output(v)) => {
                buf.advance(initial_len - temp_buf.len());
                Ok(Some(v))
            }
            Err(unbuffer::Error::NeedMoreData(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Encoder for FramedMessageCodec {
    type Error = buffer::Error;
    type Item = GenericMessage;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.required_buffer_size());
        item.buffer_ref(dst)
    }
}

pub type MessageFramed<T> = Framed<T, FramedMessageCodec>;

pub fn apply_message_framing<T: AsyncRead + AsyncWrite>(stream: T) -> MessageFramed<T> {
    Decoder::framed(FramedMessageCodec {}, stream)
}
