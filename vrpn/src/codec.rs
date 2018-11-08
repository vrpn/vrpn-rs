// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{
    base::message::SequencedGenericMessage,
    buffer::{buffer, message::MessageSize, unbuffer, Buffer, Unbuffer},
    prelude::*,
};
use pretty_hex::*;
use tokio::{
    codec::{Decoder, Encoder, Framed},
    prelude::*,
};
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FramedMessageCodec;
impl Decoder for FramedMessageCodec {
    type Item = SequencedGenericMessage;
    type Error = unbuffer::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let initial_len = buf.len();
        let size_len = u32::constant_buffer_size();
        if initial_len < size_len {
            return Ok(None);
        }
        let (combined_size, _) = buf
            .clone()
            .split_to(u32::constant_buffer_size())
            .freeze()
            .unbuffer::<u32>()
            .map_exactly_err_to_at_least()?;
        let size = MessageSize::from_unpadded_message_size(combined_size as usize);
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
        let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut temp_buf);
        eprintln!("{:?}", unbuffered);
        match unbuffered {
            Ok(v) => {
                println!("Decoder::decode has message {:?}", v);
                Ok(Some(v))
            }
            Err(unbuffer::Error::NeedMoreData(_)) => {
                unreachable!();
            }
            Err(e) => {
                buf.unsplit(taken_buf);
                Err(e)
            }
        }
    }
}

impl Encoder for FramedMessageCodec {
    type Error = buffer::Error;
    type Item = SequencedGenericMessage;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.required_buffer_size());
        item.buffer_ref(dst)
    }
}

pub type MessageFramed<T> = Framed<T, FramedMessageCodec>;

pub fn apply_message_framing<T: AsyncRead + AsyncWrite>(stream: T) -> MessageFramed<T> {
    Decoder::framed(FramedMessageCodec {}, stream)
}
