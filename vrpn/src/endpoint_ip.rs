// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::BytesMut;
use crate::{
    base::types::*,
    base::{constants::TCP_BUFLEN, Description, GenericMessage, Message},
    buffer::{buffer, make_message_body_generic, unbuffer, Buffer},
    codec::{self, FramedMessageCodec},
    connection::{
        typedispatcher::HandlerResult, TranslationTable, TranslationTableError,
        TranslationTableResult,
    },
    endpoint_channel::{EndpointChannel, EndpointError},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{
    codec::{Decoder, Encoder, Framed},
    io,
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};
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
    pub(crate) fn new(
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

    pub(crate) fn buffer_message<T: Buffer>(
        &mut self,
        msg: Message<T>,
        class: ClassOfService,
    ) -> HandlerResult<()> {
        let generic_msg = make_message_body_generic(msg)?;
        self.buffer_generic_message(generic_msg, class)
    }

    pub(crate) fn sender_table(&self) -> &TranslationTable<SenderId> {
        &self.senders
    }

    pub(crate) fn sender_table_mut(&mut self) -> &mut TranslationTable<SenderId> {
        &mut self.senders
    }

    pub(crate) fn type_table(&self) -> &TranslationTable<TypeId> {
        &self.types
    }

    pub(crate) fn type_table_mut(&mut self) -> &mut TranslationTable<TypeId> {
        &mut self.types
    }

    pub(crate) fn pack_sender_description(
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

    pub(crate) fn pack_type_description(
        &mut self,
        local_type: LocalId<TypeId>,
    ) -> TranslationTableResult<()> {
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

    /// Convert remote type ID to local type ID
    pub(crate) fn local_type_id(&self, remote_type: RemoteId<TypeId>) -> Option<LocalId<TypeId>> {
        match self.type_table().map_to_local_id(remote_type) {
            Ok(val) => val,
            Err(_) => None,
        }
    }

    /// Convert remote sender ID to local sender ID
    pub(crate) fn local_sender_id(
        &self,
        remote_sender: RemoteId<SenderId>,
    ) -> Option<LocalId<SenderId>> {
        match self.sender_table().map_to_local_id(remote_sender) {
            Ok(val) => val,
            Err(_) => None,
        }
    }

    pub(crate) fn new_local_sender(
        &mut self,
        name: SenderName,
        local_sender: LocalId<SenderId>,
    ) -> bool {
        self.sender_table_mut()
            .add_local_id(name.into(), local_sender)
    }

    pub(crate) fn new_local_type(&mut self, name: TypeName, local_type: LocalId<TypeId>) -> bool {
        self.type_table_mut().add_local_id(name.into(), local_type)
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
