// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::BytesMut;
use crate::{
    base::{
        constants::TCP_BUFLEN,
        message::{Description, GenericMessage, Message},
        types::*,
    },
    buffer::{buffer, unbuffer, Buffer},
    codec::{self, FramedMessageCodec},
    connection::{
        typedispatcher::HandlerResult, Endpoint, TranslationTable, TranslationTableError,
        TranslationTableResult,
    },
    endpoint_channel::{EndpointChannel, EndpointError},
};
use bytes::BytesMut;
use tokio::{
    codec::{Decoder, Encoder, Framed},
    io,
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};
use std::sync::atomic::{AtomicUsize, Ordering};
pub type MessageFramed = codec::MessageFramed<TcpStream>;
pub type MessageFramedUdp = UdpFramed<FramedMessageCodec>;

pub struct EndpointIP {
    types: TranslationTable<TypeId>,
    senders: TranslationTable<SenderId>,
    wr: BytesMut,
    reliable_channel: EndpointChannel<MessageFramed>,
    seq: AtomicUsize, // low_latency_tx: Option<MessageFramedUdp>
}

impl EndpointIP {
    pub fn new(
        reliable_stream: TcpStream //low_latency_channel: Option<MessageFramedUdp>
    ) -> EndpointIP {
        let framed = codec::apply_message_framing(reliable_stream);
        EndpointIP {
            types: TranslationTable::new(),
            senders: TranslationTable::new(),
            wr: BytesMut::new(),
            reliable_channel: EndpointChannel::new(framed, TCP_BUFLEN),
            seq: AtomicUsize::new(0)
            // low_latency_channel,
        }
    }
}
impl Endpoint for EndpointIP {
    fn buffer_generic_message(
        &mut self,
        msg: GenericMessage,
        class: ClassOfService,
    ) -> HandlerResult<()> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        self.reliable_channel
            .buffer(msg.add_sequence_number(SequenceNumber(seq as u32)))?;
        unimplemented!();
    }

    fn sender_table(&self) -> &TranslationTable<SenderId> {
        &self.senders
    }
    fn sender_table_mut(&mut self) -> &mut TranslationTable<SenderId> {
        &mut self.senders
    }
    fn type_table(&self) -> &TranslationTable<TypeId> {
        &self.types
    }
    fn type_table_mut(&mut self) -> &mut TranslationTable<TypeId> {
        &mut self.types
    }

    fn pack_sender_description(
        &mut self,
        local_sender: LocalId<SenderId>,
    ) -> TranslationTableResult<()> {
        let LocalId(id) = local_sender;
        let name = self
            .senders
            .find_by_local_id(local_sender)
            .ok_or(TranslationTableError::InvalidLocalId(id.get()))
            .and_then(|entry| Ok(entry.name.clone()))?;
        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map_err(|e| TranslationTableError::HandlerError(e))
            .map(|_| ())
    }

    fn pack_type_description(&mut self, local_type: LocalId<TypeId>) -> TranslationTableResult<()> {
        let LocalId(id) = local_type;
        let name = self
            .types
            .find_by_local_id(local_type)
            .ok_or(TranslationTableError::InvalidLocalId(id.get()))
            .and_then(|entry| Ok(entry.name.clone()))?;

        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map_err(|e| TranslationTableError::HandlerError(e))
            .map(|_| ())
    }
}

impl Future for EndpointIP {
    type Item = ();
    type Error = EndpointError;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.reliable_channel.poll_channel(|msg| {
            eprintln!("Received message {:?}", msg);
            // todo do something here
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn make_endpoint() {
        use crate::connect::connect_tcp;
        let addr = "127.0.0.1:3883".parse().unwrap();
        let _ = connect_tcp(addr)
            .and_then(|stream| {
                let mut ep = EndpointIP::new(stream);
                // future::poll_fn(move || ep.poll())
                // .map_err(|e| {
                //     eprintln!("{}", e);
                //     panic!()
                // })
                let _ = ep.poll().unwrap();
                let _ = ep.poll().unwrap();
                let _ = ep.poll().unwrap();
                Ok(())
            })
            .wait()
            .unwrap();
    }
}
