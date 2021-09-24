// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::Buf;

use crate::{
    buffer_unbuffer::{BufferUnbufferError, UnbufferResult},
    data_types::SequencedGenericMessage,
};

/// Decode at most 1 message. Returns Ok(None) if we don't have enough data.
pub(crate) fn maybe_decode_one<T: Buf + Clone>(
    buf: &mut T,
) -> UnbufferResult<Option<SequencedGenericMessage>> {
    match SequencedGenericMessage::try_read_from_buf(buf) {
        Ok(v) => Ok(Some(v)),
        // Not enough data in the buffer - here, that's not an error.
        Err(BufferUnbufferError::NeedMoreData(_)) => Ok(None),
        Err(e) => Err(e),
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
    use bytes::Bytes;

    use super::*;

    #[test]
    fn individual_decode_one() {
        const MSG1: [u8; 48] = hex!(
            // length is 0x29 = 41
            "00 00 00 29"
            // timestamp seconds 1542140718
            "5b eb 33 2e"
            // timestamp useconds 809137
            "00 0c 58 b1"
            // sender 0
            "00 00 00 00"
            // message type -1
            "ff ff ff ff"
            // sequence/padding
            "00 00 00 00"
            // body
            "00 00 00 0d 56 52 50 4e 20 43 6f 6e 74 72 6f 6c 00 00 00 00 00 00 00 00");
        const MSG2: [u8; 40] = hex!(
            // length is 0x25 = 37
            "00 00 00 25"
            // timestamp seconds 1542140718
            "5b eb 33 2e"
            // timestamp useconds 809137
            "00 0c 58 b1"
            // sender 1
            "00 00 00 01"
            // message type -1
            "ff ff ff ff"
            // sequence/padding
            "00 00 00 01"
            // body
            "00 00 00 09 54 72 61 63 6b 65 72 30 00 00 00 00");
        const MSG3: [u8; 72] = hex!(
            // length is 0x41 = 65
            "00 00 00 41"
            // timestamp seconds 1542140718
            "5b eb 33 2e"
            // timestamp useconds 809138
            "00 0c 58 b2"
            // sender 0
            "00 00 00 00"
            // message type
            "ff ff ff fe"
            // sequence/padding
            "00 00 00 02"
            // body
            "00 00 00 25 56 52 50 4e 5f 43 6f 6e 6e 65 63 74 69 6f 6e 5f 47 6f 74 5f 46 69 72 73 74 5f 43 6f 6e 6e 65 63 74 69 6f 6e 00 00 00 00 00 00 00 00");

        // const test_messages = ;
        for msg_bytes in [Vec::from(MSG1), Vec::from(MSG2), Vec::from(MSG3)] {
            let mut data = Bytes::copy_from_slice(&msg_bytes);
            let decoded = maybe_decode_one(&mut data);
            assert!(decoded.is_ok());
            let decoded = decoded.unwrap();
            assert!(decoded.is_some());
            let _decoded = decoded.unwrap();
            assert_eq!(data.len(), 0);
        }
    }
}
