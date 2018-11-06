// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
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
    endpoint_channel::{poll_channel, EndpointChannel, EndpointError},
};
use bytes::BytesMut;
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
    reliable_channel: EndpointChannel<TcpStream>,
    // low_latency_tx: Option<MessageFramedUdp>,
}

impl EndpointIP {
    pub fn new(
        reliable_stream: TcpStream //low_latency_channel: Option<MessageFramedUdp>
    ) -> EndpointIP {
        EndpointIP {
            types: TranslationTable::new(),
            senders: TranslationTable::new(),
            wr: BytesMut::new(),
            reliable_channel: EndpointChannel::new(reliable_stream, TCP_BUFLEN),
            // low_latency_channel,
        }
    }
}
impl Endpoint for EndpointIP {
    fn send_message(
        &mut self,
        time: Time,
        id: TypeId,
        sender: SenderId,
        buffer: bytes::Bytes,
        class: ClassOfService,
    ) -> HandlerResult<()> {
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
        self.wr.reserve(desc_msg.required_buffer_size());
        desc_msg
            .buffer_ref(&mut self.wr)
            .map_err(|e| TranslationTableError::BufferError(e))
    }

    fn pack_type_description(&mut self, local_type: LocalId<TypeId>) -> TranslationTableResult<()> {
        let LocalId(id) = local_type;
        let name = self
            .types
            .find_by_local_id(local_type)
            .ok_or(TranslationTableError::InvalidLocalId(id.get()))
            .and_then(|entry| Ok(entry.name.clone()))?;

        let desc_msg = Message::from(Description::new(id, name));
        self.wr.reserve(desc_msg.required_buffer_size());
        desc_msg
            .buffer_ref(&mut self.wr)
            .map_err(|e| TranslationTableError::BufferError(e))
    }
}

impl Future for EndpointIP {
    type Item = Option<GenericMessage>;
    type Error = EndpointError;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        poll_channel(&mut self.reliable_channel)
    }
}
