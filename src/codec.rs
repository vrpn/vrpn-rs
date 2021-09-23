// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Buf, Bytes, BytesMut};

use crate::{
    buffer_unbuffer::{check_unbuffer_remaining, peek_u32, BufferUnbufferError, UnbufferResult},
    data_types::{MessageSize, SequencedGenericMessage},
    Result,
};

pub(crate) fn peek_u32_bytes_mut(buf: &BytesMut) -> Result<Option<u32>> {
    const SIZE_LEN: usize = std::mem::size_of::<u32>();
    if buf.remaining() < SIZE_LEN {
        eprintln!("Not enough remaining bytes for the size.");
        return Ok(None);
    }
    let peeked = (&buf[..SIZE_LEN]).get_u32();
    Ok(Some(peeked))
}

/// Decode exactly 1 message. Returns Ok(None) if we don't have enough data.
pub(crate) fn decode_one<T: Buf>(buf: &mut T) -> UnbufferResult<Option<SequencedGenericMessage>> {
    // Peek the length field if possible
    if let Some(combined_size) = peek_u32(buf) {
        let size = MessageSize::from_length_field(combined_size);
        if check_unbuffer_remaining(buf, size.padded_message_size()).is_err() {
            // Not enough data in the buffer - here, that's not an error.
            return Ok(None);
        }

        // Make an interface to take exactly what we need from the buffer
        let mut taken_buf = buf.take(size.padded_message_size());
        let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut taken_buf);
        match unbuffered {
            Ok(v) => {
                buf.advance(size.padded_message_size());
                Ok(Some(v))
            }
            Err(BufferUnbufferError::NeedMoreData(_)) => {
                unreachable!();
            }
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
}

// pub(crate) fn decode_one_mut(buf: &mut BytesMut) -> Result<Option<SequencedGenericMessage>> {
//     let initial_len = buf.len();
//     if let Some(combined_size) = peek_u32_bytes_mut(buf)? {
//         let mut inner_buf = buf.clone();
//         let size = MessageSize::from_length_field(combined_size);
//         if initial_len < size.padded_message_size() {
//             return Ok(None);
//         }
//         let mut taken_buf = inner_buf.split_to(size.padded_message_size());
//         let unbuffered = SequencedGenericMessage::unbuffer_ref(&mut taken_buf);
//         match unbuffered {
//             Ok(v) => {
//                 Buf::advance(&mut buf, size.padded_message_size());
//                 Ok(Some(v))
//             }
//             Err(BufferUnbufferError::NeedMoreData(_)) => {
//                 unreachable!();
//             }
//             Err(e) => Err(e),
//         }
//     } else {
//         Ok(None)
//     }
// }
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek() {
        use crate::codec::peek_u32;
        use bytes::Buf;
        let data = b"\0\0\0\0";
        {
            let buf = &data[..];
            assert_eq!(peek_u32(&buf), Some(0));
            assert_eq!(buf.remaining(), data.len());
        }
    }
    #[test]
    fn individual_decode_one() {
        const MSG1: [u8; 48]= hex!("00 00 00 29 5b eb 33 2e 00 0c 58 b1 00 00 00 00 ff ff ff ff 00 00 00 00 00 00 00 0d 56 52 50 4e 20 43 6f 6e 74 72 6f 6c 00 00 00 00 00 00 00 00");
        const MSG2: [u8; 40] = hex!("00 00 00 25 5b eb 33 2e 00 0c 58 b1 00 00 00 01 ff ff ff ff 00 00 00 01 00 00 00 09 54 72 61 63 6b 65 72 30 00 00 00 00");
        const MSG3: [u8; 72] = hex!("00 00 00 41 5b eb 33 2e 00 0c 58 b2 00 00 00 00 ff ff ff fe 00 00 00 02 00 00 00 25 56 52 50 4e 5f 43 6f 6e 6e 65 63 74 69 6f 6e 5f 47 6f 74 5f 46 69 72 73 74 5f 43 6f 6e 6e 65 63 74 69 6f 6e 00 00 00 00 00 00 00 00");

        // const test_messages = ;
        for msg_bytes in [Vec::from(MSG1), Vec::from(MSG2), Vec::from(MSG3)] {
            let mut data = Bytes::copy_from_slice(&msg_bytes);
            let decoded = decode_one(&mut data);
            assert!(decoded.is_ok());
            let decoded = decoded.unwrap();
            assert!(decoded.is_some());
            let _decoded = decoded.unwrap();
            assert_eq!(data.len(), 0);
        }
    }
}
