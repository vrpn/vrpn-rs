// Copyright 2018-2019, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{codec::decode_one, Buffer, Error, Result, SequencedGenericMessage};
use bytes::{Bytes, BytesMut};
use tokio::{
    prelude::*,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FramedMessageCodec;
impl Decoder for FramedMessageCodec {
    type Item = SequencedGenericMessage;
    type Error = Error;
    fn decode(&mut self, buf: &mut Bytes) -> Result<Option<Self::Item>> {
        let initial_len = buf.len();
        if initial_len == 0 {
            // short-circuit if we have run out of stuff.
            return Ok(None);
        }
        let mut inner_buf = Bytes::from(buf.clone());
        match decode_one(&mut inner_buf)? {
            Some(msg) => {
                let consumed = initial_len - inner_buf.len();
                buf.advance(consumed);
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}

impl Encoder for FramedMessageCodec {
    type Error = Error;
    type Item = SequencedGenericMessage;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<()> {
        dst.reserve(item.required_buffer_size());
        item.buffer_ref(dst)
    }
}

pub type MessageFramed<T> = Framed<T, FramedMessageCodec>;

pub fn apply_message_framing<T: AsyncRead + AsyncWrite>(stream: T) -> MessageFramed<T> {
    Decoder::framed(FramedMessageCodec {}, stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{descriptions::InnerDescription, Message, SenderId};
    use bytes::BufMut;
    type SenderInnerDesc = Message<InnerDescription<SenderId>>;

    fn to_sender_inner_desc(msg: &SequencedGenericMessage) -> SenderInnerDesc {
        let msg = Message::from(msg.clone());
        SenderInnerDesc::try_from_generic(&msg).unwrap()
    }
    fn get_test_messages() -> Vec<Vec<u8>> {
        vec![
            Vec::from(&hex!("00 00 00 29 5b eb 33 2e 00 0c 58 b1 00 00 00 00 ff ff ff ff 00 00 00 00 00 00 00 0d 56 52 50 4e 20 43 6f 6e 74 72 6f 6c 00 00 00 00 00 00 00 00")[..]),
            Vec::from(&hex!("00 00 00 25 5b eb 33 2e 00 0c 58 b1 00 00 00 01 ff ff ff ff 00 00 00 01 00 00 00 09 54 72 61 63 6b 65 72 30 00 00 00 00")[..]),
            Vec::from(&hex!("00 00 00 41 5b eb 33 2e 00 0c 58 b2 00 00 00 00 ff ff ff fe 00 00 00 02 00 00 00 25 56 52 50 4e 5f 43 6f 6e 6e 65 63 74 69 6f 6e 5f 47 6f 74 5f 46 69 72 73 74 5f 43 6f 6e 6e 65 63 74 69 6f 6e 00 00 00 00 00 00 00 00")[..])
        ]
    }
    #[test]
    fn individual_decode() {
        for msg_bytes in &get_test_messages() {
            let mut data = BytesMut::from(&msg_bytes[..]);
            let decoded = FramedMessageCodec.decode(&mut data);
            assert!(decoded.is_ok());
            let decoded = decoded.unwrap();
            assert!(decoded.is_some());
            assert_eq!(data.remaining_mut(), 0);
        }
    }

    #[test]
    fn decode_multiple() {
        let mut all_bytes = Vec::new();
        for msg_bytes in get_test_messages() {
            all_bytes.append(&mut msg_bytes.clone());
        }
        let mut data = BytesMut::from(&all_bytes[..]);
        let mut decoded = Vec::new();
        decoded.push(FramedMessageCodec.decode(&mut data).unwrap().unwrap());
        decoded.push(FramedMessageCodec.decode(&mut data).unwrap().unwrap());
        decoded.push(FramedMessageCodec.decode(&mut data).unwrap().unwrap());

        assert_eq!(
            &to_sender_inner_desc(&decoded[0]).body.name[..],
            &b"VRPN Control"[..]
        );
        assert_eq!(
            &to_sender_inner_desc(&decoded[1]).body.name[..],
            &b"Tracker0"[..]
        );

        assert_eq!(
            &to_sender_inner_desc(&decoded[2]).body.name[..],
            &b"VRPN_Connection_Got_First_Connection"[..]
        );
    }
}
