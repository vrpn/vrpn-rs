// Copyright 2018-2019, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::prelude::*;
use crate::{message::MessageSize, Buffer, Error, Result, SequencedGenericMessage, Unbuffer};
use bytes::{Buf, Bytes, BytesMut, IntoBuf};


pub fn peek_u32(buf: &Bytes) -> Result<Option<u32>> {
    let size_len = u32::constant_buffer_size();
    if buf.len() < size_len {
        eprintln!("Not enough remaining bytes for the size.");
        return Ok(None);
    }
    let peeked = buf[..size_len].into_buf().get_u32_be();
    Ok(Some(peeked))
}

pub(crate) fn decode_one(buf: &mut Bytes) -> Result<Option<SequencedGenericMessage>> {
    let initial_len = buf.len();
    if let Some(combined_size) = peek_u32(buf)? {
        let mut inner_buf = buf.clone();
        let size = MessageSize::from_length_field(combined_size);
        if initial_len < size.padded_message_size() {
            return Ok(None);
        }
        let mut taken_buf = inner_buf.split_to(size.padded_message_size());
        let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut taken_buf);
        match unbuffered {
            Ok(v) => {
                buf.advance(size.padded_message_size());
                Ok(Some(v))
            }
            Err(Error::NeedMoreData(_)) => {
                unreachable!();
            }
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
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
    fn individual_decode_one() {
        for msg_bytes in &get_test_messages() {
            let mut data = Bytes::from(&msg_bytes[..]);
            let decoded = decode_one(&mut data);
            assert!(decoded.is_ok());
            let decoded = decoded.unwrap();
            assert!(decoded.is_some());
            let _decoded = decoded.unwrap();
            assert_eq!(data.len(), 0);
        }
    }
}
