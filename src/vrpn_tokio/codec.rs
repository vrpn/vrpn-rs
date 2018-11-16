// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::prelude::*;
use crate::{message::MessageSize, Buffer, Error, Result, SequencedGenericMessage, Unbuffer};
use tokio::{
    codec::{Decoder, Encoder, Framed},
    prelude::*,
};

fn decode_one(buf: &mut Bytes) -> Result<Option<SequencedGenericMessage>> {
    let initial_len = buf.len();
    let size_len = u32::constant_buffer_size();
    if initial_len < size_len {
        eprintln!("Not enough remaining bytes for the size.");
        return Ok(None);
    }
    let mut inner_buf = buf.clone();
    let (combined_size, _) = inner_buf
        .clone()
        .split_to(u32::constant_buffer_size())
        .unbuffer::<u32>()
        .map_exactly_err_to_at_least()?;
    let size = MessageSize::from_unpadded_message_size(combined_size as usize);
    if initial_len < size.padded_message_size() {
        eprintln!(
            "Not enough remaining bytes for the message: have {}, need {}.",
            initial_len,
            size.padded_message_size()
        );
        return Ok(None);
    }
    let mut taken_buf = inner_buf.split_to(size.padded_message_size());
    let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut taken_buf);
    match unbuffered {
        Ok(v) => {
            // println!("Decoder::decode has message {:?}", v);
            eprintln!("Buffer now has {:?} bytes", buf.len());
            buf.advance(size.padded_message_size());
            Ok(Some(v))
        }
        Err(Error::NeedMoreData(_)) => {
            unreachable!();
        }
        Err(e) => Err(e),
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FramedMessageCodec;
impl Decoder for FramedMessageCodec {
    type Item = Vec<SequencedGenericMessage>;
    type Error = Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
        let initial_len = buf.len();
        let mut inner_buf = Bytes::from(buf.clone());
        let mut messages = Vec::new();
        while let Some(msg) = decode_one(&mut inner_buf)? {
            messages.push(msg);
        }
        if messages.is_empty() {
            Ok(None)
        } else {
            let consumed = initial_len - inner_buf.len();
            eprintln!(
                "Consumed {} bytes for {} messages",
                consumed,
                messages.len()
            );
            buf.advance(consumed);
            Ok(Some(messages))
        }
    }
}

impl Encoder for FramedMessageCodec {
    type Error = Error;
    type Item = SequencedGenericMessage;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<()> {
        // for msg in item.into_iter() {
        //     dst.reserve(msg.required_buffer_size());
        //     msg.buffer_ref(dst)?;
        // }
        // Ok(())

        dst.reserve(item.required_buffer_size());
        item.buffer_ref(dst)
    }
}

pub type MessageFramed<T> = Framed<T, FramedMessageCodec>;

pub fn apply_message_framing<T: AsyncRead + AsyncWrite>(stream: T) -> MessageFramed<T> {
    Decoder::framed(FramedMessageCodec {}, stream)
}
