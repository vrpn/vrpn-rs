// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{
    base::types::*,
    base::{
        Description, Error, GenericMessage, InnerDescription, Message, Result, TypedMessageBody,
    },
    codec::{self, FramedMessageCodec},
    connection::{endpoint::*, MatchingTable, TranslationTables, TypeDispatcher},
    endpoint_channel::{poll_and_dispatch, EndpointChannel},
};
use futures::sync::mpsc;
use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    net::{TcpStream, UdpFramed},
    prelude::*,
};

pub type MessageFramed = codec::MessageFramed<TcpStream>;
pub type MessageFramedUdp = UdpFramed<FramedMessageCodec>;

#[derive(Debug)]
pub struct EndpointIp {
    translation: TranslationTables,
    wr: BytesMut,
    reliable_channel: Arc<Mutex<EndpointChannel<MessageFramed>>>,
    low_latency_channel: Option<()>,
    seq: AtomicUsize,
    system_rx: mpsc::UnboundedReceiver<SystemMessage>,
    system_tx: mpsc::UnboundedSender<SystemMessage>,
}
impl EndpointIp {
    pub(crate) fn new(
        reliable_stream: TcpStream //low_latency_channel: Option<MessageFramedUdp>
    ) -> EndpointIp {
        let framed = codec::apply_message_framing(reliable_stream);
        let (system_tx, system_rx) = mpsc::unbounded();
        EndpointIp {
            translation: TranslationTables::new(),
            wr: BytesMut::new(),
            reliable_channel: EndpointChannel::new(framed),
            low_latency_channel: None,
            seq: AtomicUsize::new(0),
            system_tx,
            system_rx,
        }
    }

    pub(crate) fn pack_description<T>(&mut self, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let LocalId(id) = local_id;
        let name = self
            .translation
            .find_by_local_id(local_id)
            .ok_or_else(|| Error::InvalidId(id.get()))
            .and_then(|entry| Ok(entry.name().clone()))?;
        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map(|_| ())
    }

    pub(crate) fn clear_other_senders_and_types(&mut self) {
        self.translation.clear();
    }

    /// Convert remote sender/type ID to local sender/type ID
    pub(crate) fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: BaseTypeSafeId,
        TranslationTables: MatchingTable<T>,
    {
        match self.translation.map_to_local_id(remote_id) {
            Ok(val) => val,
            Err(_) => None,
        }
    }
    pub(crate) fn new_local_id<T, U>(&mut self, name: U, local_id: LocalId<T>) -> bool
    where
        T: BaseTypeSafeIdName + BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
        U: Into<<T as BaseTypeSafeIdName>::Name>,
    {
        let name: <T as BaseTypeSafeIdName>::Name = name.into();
        let name: Bytes = name.into();
        self.translation.add_local_id(name, local_id)
    }

    pub(crate) fn poll_endpoint(&mut self, dispatcher: &mut TypeDispatcher) -> Poll<(), Error> {
        let channel_arc = Arc::clone(&self.reliable_channel);
        let mut channel = channel_arc
            .lock()
            .map_err(|e| Error::OtherMessage(e.to_string()))?;
        let _ = channel.poll_complete()?;
        let closed = poll_and_dispatch(self, channel.deref_mut(), dispatcher)?.is_ready();

        // todo UDP here.

        if closed {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl Endpoint for EndpointIp {
    fn send_system_change(&self, message: SystemMessage) -> Result<()> {
        println!("send_system_change {:?}", message);
        self.system_tx
            .unbounded_send(message)
            .map_err(|e| Error::OtherMessage(e.to_string()))?;
        Ok(())
    }

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()> {
        if class.contains(ServiceFlags::RELIABLE) || self.low_latency_channel.is_none() {
            // We either need reliable, or don't have low-latency
            let mut channel = self
                .reliable_channel
                .lock()
                .map_err(|e| Error::OtherMessage(e.to_string()))?;
            match channel
                .start_send(msg)
                .map_err(|e| Error::OtherMessage(e.to_string()))?
            {
                AsyncSink::Ready => Ok(()),
                AsyncSink::NotReady(_) => Err(Error::OtherMessage(String::from(
                    "Didn't have room in send buffer",
                ))),
            }
        } else {
            // have and can use low-latency
            unimplemented!()
        }
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
                let ep = EndpointIp::new(stream);
                for _i in 0..4 {
                    let _ = ep
                        .reliable_channel
                        .lock()
                        .unwrap()
                        .poll()
                        .unwrap()
                        .map(|msg| {
                            eprintln!("Received message {:?}", msg);
                            msg
                        });
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }
    #[test]
    fn run_endpoint() {
        use crate::connect::connect_tcp;
        let addr = "127.0.0.1:3883".parse().unwrap();
        let _ = connect_tcp(addr)
            .and_then(|stream| {
                let mut ep = EndpointIp::new(stream);
                let mut disp = TypeDispatcher::new();
                for _i in 0..4 {
                    let _ = ep.poll_endpoint(&mut disp).unwrap();
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }
}
