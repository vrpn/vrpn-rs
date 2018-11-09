// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{
    base::types::*,
    base::{constants::TCP_BUFLEN, Description, GenericMessage, Message},
    buffer::{buffer, make_message_body_generic, unbuffer, Buffer},
    codec::{self, FramedMessageCodec},
    connection::{
        Endpoint, Error as ConnectionError, Result as ConnectionResult, TranslationTable,
    },
    endpoint_channel::{EndpointChannel, EpSinkError, EpSinkItem, EpStreamError, EpStreamItem},
    error::Error,
    inner_lock, ArcConnectionIpInner,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    codec::{Decoder, Encoder, Framed},
    io,
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};
pub type MessageFramed = codec::MessageFramed<TcpStream>;
pub type MessageFramedUdp = UdpFramed<FramedMessageCodec>;

#[derive(Debug)]
pub struct EndpointIp {
    pub(crate) types: TranslationTable<TypeId>,
    pub(crate) senders: TranslationTable<SenderId>,
    wr: BytesMut,
    reliable_channel: EndpointChannel<MessageFramed>,
    seq: AtomicUsize, // low_latency_tx: Option<MessageFramedUdp>
}

impl EndpointIp {
    pub(crate) fn new(
        reliable_stream: TcpStream //low_latency_channel: Option<MessageFramedUdp>
    ) -> EndpointIp {
        let framed = codec::apply_message_framing(reliable_stream);
        EndpointIp {
            types: TranslationTable::new(),
            senders: TranslationTable::new(),
            wr: BytesMut::new(),
            reliable_channel: EndpointChannel::new(framed),
            seq: AtomicUsize::new(0)
            // low_latency_channel,
        }
    }
    fn buffer_generic_message(
        &mut self,
        msg: GenericMessage,
        class: ClassOfService,
    ) -> ConnectionResult<()> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        self.reliable_channel
            .buffer(msg.add_sequence_number(SequenceNumber(seq as u32)))?;
        unimplemented!();
    }

    pub(crate) fn buffer_message<T: Buffer>(
        &mut self,
        msg: Message<T>,
        class: ClassOfService,
    ) -> ConnectionResult<()> {
        let generic_msg = make_message_body_generic(msg)?;
        self.buffer_generic_message(generic_msg, class)
    }

    pub(crate) fn sender_table(&self) -> &TranslationTable<SenderId> {
        &self.senders
    }

    pub(crate) fn type_table(&self) -> &TranslationTable<TypeId> {
        &self.types
    }

    pub(crate) fn pack_sender_description(
        &mut self,
        local_sender: LocalId<SenderId>,
    ) -> ConnectionResult<()> {
        let LocalId(id) = local_sender;
        let name = self
            .senders
            .find_by_local_id(local_sender)
            .ok_or(ConnectionError::InvalidLocalId(id.get()))
            .and_then(|entry| Ok(entry.name.clone()))?;
        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map(|_| ())
    }

    pub(crate) fn clear_other_senders_and_types(&mut self) {
        self.senders.clear();
        self.types.clear();
    }

    pub(crate) fn pack_type_description(
        &mut self,
        local_type: LocalId<TypeId>,
    ) -> ConnectionResult<()> {
        let LocalId(id) = local_type;
        let name = self
            .types
            .find_by_local_id(local_type)
            .ok_or(ConnectionError::InvalidLocalId(id.get()))
            .and_then(|entry| Ok(entry.name.clone()))?;

        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
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
        name: impl Into<SenderName>,
        local_sender: LocalId<SenderId>,
    ) -> bool {
        let name: SenderName = name.into();
        self.sender_table_mut()
            .add_local_id(name.into(), local_sender)
    }

    pub(crate) fn new_local_type(
        &mut self,
        name: impl Into<TypeName>,
        local_type: LocalId<TypeId>,
    ) -> bool {
        let name: TypeName = name.into();
        self.type_table_mut().add_local_id(name.into(), local_type)
    }

    pub(crate) fn poll_endpoint(&mut self, conn: ArcConnectionIpInner) -> Poll<(), Error> {
        let channel = &mut self.reliable_channel;
        let closed = channel
            .process_send_receive(|msg| -> Result<(), Error> {
                eprintln!("Received message {:?}", msg);
                if msg.header.message_type.is_system_message() {
                    let mut conn = inner_lock::<Error>(&conn)?;
                    conn.type_dispatcher.do_system_callbacks_for(&msg, self)?;
                } else {
                    let mut conn = inner_lock::<Error>(&conn)?;
                    conn.type_dispatcher.do_callbacks_for(&msg)?;
                }
                // todo do something here
                Ok(())
            })?
            .is_ready();

        // todo UDP here.

        if closed {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl Endpoint for EndpointIp {
    fn connect_to_udp(&mut self, addr: Bytes, port: usize) -> ConnectionResult<()> {
        eprintln!("Told to connect over UDP to {:?}:{}", addr, port);
        Ok(())
    }
    fn sender_table_mut(&mut self) -> &mut TranslationTable<SenderId> {
        &mut self.senders
    }
    fn type_table_mut(&mut self) -> &mut TranslationTable<TypeId> {
        &mut self.types
    }
}

impl Future for EndpointIp {
    type Item = ();
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.reliable_channel.process_send_receive(|msg| {
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
                let mut ep = EndpointIp::new(stream);
                // future::poll_fn(move || ep.poll())
                // .map_err(|e| {
                //     eprintln!("{}", e);
                //     panic!()
                // })
                for _i in 0..4 {
                    let _ = ep
                        .reliable_channel
                        .process_send_receive(|msg| {
                            eprintln!("Received message {:?}", msg);
                            Ok(())
                        })
                        .unwrap();
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }
}
