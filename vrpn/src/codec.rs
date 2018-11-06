// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    base::message::GenericMessage,
    buffer::{buffer, message::MessageSize, unbuffer, Buffer, Output, Unbuffer},
    prelude::*,
};
use bytes::BytesMut;
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
        let mut temp_buf = BytesMut::clone(buf).freeze();
        let combined_size = u32::unbuffer_ref(&mut temp_buf)
            .map_exactly_err_to_at_least()?
            .data() as usize;
        let size = MessageSize::from_unpadded_message_size(combined_size);
        if initial_len < size.padded_message_size() {
            return Ok(None);
        }
        let mut temp_buf = BytesMut::clone(buf).freeze();
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
