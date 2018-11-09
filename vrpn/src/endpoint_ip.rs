// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use bytes::{Bytes, BytesMut};
use crate::{
    base::types::*,
    base::{
        constants::{self, TCP_BUFLEN},
        Description, Error, GenericMessage, InnerDescription, Message, Result, TypedMessageBody,
    },
    buffer::{buffer, make_message_body_generic, unbuffer, unbuffer_typed_message_body, Buffer},
    codec::{self, FramedMessageCodec},
    connection::{
        endpoint::*, translation, MatchingTable, TranslationTable, TranslationTables,
        TypeDispatcher,
    },
    endpoint_channel::{EndpointChannel, EpSinkError, EpSinkItem, EpStreamError, EpStreamItem},
    inner_lock, ArcConnectionIpInner,
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
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};

pub type MessageFramed = codec::MessageFramed<TcpStream>;
pub type MessageFramedUdp = UdpFramed<FramedMessageCodec>;

/// Given a stream of GenericMessage, poll the stream and dispatch received messages.
fn poll_and_dispatch<T>(
    endpoint: &mut EndpointIp,
    stream: &mut T,
    dispatcher: &mut TypeDispatcher,
) -> Poll<(), Error>
where
    T: Stream<Item = GenericMessage, Error = Error>,
{
    const MAX_PER_TICK: usize = 10;
    let mut closed = false;
    for i in 0..MAX_PER_TICK {
        match stream.poll()? {
            Async::Ready(Some(msg)) => {
                eprintln!("Received message {:?}", msg);
                if msg.is_system_message() {
                    endpoint.handle_system_message(msg)?;
                } else {
                    dispatcher.do_callbacks_for(&msg)?;
                }
            }
            Async::Ready(None) => {
                // connection closed
                closed = true;
                break;
            }
            Async::NotReady => {
                break;
            }
        }
        // If this is the last iteration, the loop will break even
        // though there could still be messages to read. Because we did
        // not reach `Async::NotReady`, we have to notify ourselves
        // in order to tell the executor to schedule the task again.
        if i + 1 == MAX_PER_TICK {
            task::current().notify();
        }
    }
    if closed {
        Ok(Async::Ready(()))
    } else {
        Ok(Async::NotReady)
    }
}
// fn poll_and_dispatch_channel<T>(
//     endpoint: &mut EndpointIp,
//     channel: &mut EndpointChannel<T>,
//     dispatcher: &mut TypeDispatcher,
// ) -> Poll<(), Error>
// where
//     T: Sink<SinkItem = EpSinkItem, SinkError = EpSinkError>
//         + Stream<Item = EpStreamItem, Error = EpStreamError>,
// {
//     let mut closed = channel.poll_flush()?.is_ready();

//     const MAX_PER_TICK: usize = 10;
//     for i in 0..MAX_PER_TICK {
//         match channel.poll_receive()? {
//             Async::Ready(Some(msg)) => {
//                 eprintln!("Received message {:?}", msg);
//                 if msg.is_system_message() {
//                     endpoint.handle_system_message(msg)?;
//                 } else {
//                     dispatcher.do_callbacks_for(&msg)?;
//                 }
//             }
//             Async::Ready(None) => {
//                 // connection closed
//                 closed = true;
//             }
//             Async::NotReady => {
//                 break;
//             }
//         }
//         // If this is the last iteration, the loop will break even
//         // though there could still be messages to read. Because we did
//         // not reach `Async::NotReady`, we have to notify ourselves
//         // in order to tell the executor to schedule the task again.
//         if i + 1 == MAX_PER_TICK {
//             task::current().notify();
//         }
//     }
//     if closed {
//         Ok(Async::Ready(()))
//     } else {
//         Ok(Async::NotReady)
//     }
// }

#[derive(Debug)]
pub struct EndpointIp {
    translation: TranslationTables,
    wr: BytesMut,
    reliable_channel: Arc<Mutex<EndpointChannel<MessageFramed>>>,
    seq: AtomicUsize,
    system_rx: mpsc::UnboundedReceiver<SystemMessage>,
    system_tx: mpsc::UnboundedSender<SystemMessage>, // low_latency_tx: Option<MessageFramedUdp>
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
            seq: AtomicUsize::new(0),
            system_tx,
            system_rx
            // low_latency_channel,
        }
    }

    // pub(crate) fn sender_table(&self) -> &TranslationTable<SenderId> {
    //     &self.senders
    // }

    // pub(crate) fn type_table(&self) -> &TranslationTable<TypeId> {
    //     &self.types
    // }

    pub(crate) fn pack_description<T>(&mut self, local_id: LocalId<T>) -> Result<()>
    where
        T: BaseTypeSafeId,
        InnerDescription<T>: TypedMessageBody,
        TranslationTables: MatchingTable<T>,
    {
        let LocalId(id) = local_id;
        let name = translation::find_by_local_id(&mut self.translation, local_id)
            .ok_or_else(|| Error::InvalidId(id.get()))
            .and_then(|entry| Ok(entry.name().clone()))?;
        let desc_msg = Message::from(Description::new(id, name));
        self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
            .map(|_| ())
    }

    pub(crate) fn clear_other_senders_and_types(&mut self) {
        self.translation.clear();
    }

    // pub(crate) fn pack_type_description(&mut self, local_type: LocalId<TypeId>) -> Result<()> {
    //     let LocalId(id) = local_type;
    //     let name = self
    //         .translation
    //         .types
    //         .find_by_local_id(local_type)
    //         .ok_or_else(|| Error::InvalidId(id.get()))
    //         .and_then(|entry| Ok(entry.name().clone()))?;

    //     let desc_msg = Message::from(Description::new(id, name));
    //     self.buffer_message(desc_msg, ClassOfService::from(ServiceFlags::RELIABLE))
    //         .map(|_| ())
    // }

    /// Convert remote sender/type ID to local sender/type ID
    pub(crate) fn map_to_local_id<T>(&self, remote_id: RemoteId<T>) -> Option<LocalId<T>>
    where
        T: BaseTypeSafeId,
        TranslationTables: MatchingTable<T>,
    {
        match translation::map_to_local_id(&self.translation, remote_id) {
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

    pub(crate) fn new_local_sender(
        &mut self,
        name: impl Into<SenderName>,
        local_sender: LocalId<SenderId>,
    ) -> bool {
        let name: SenderName = name.into();
        self.translation
            .senders
            .add_local_id(name.into(), local_sender)
    }

    pub(crate) fn new_local_type(
        &mut self,
        name: impl Into<TypeName>,
        local_type: LocalId<TypeId>,
    ) -> bool {
        let name: TypeName = name.into();
        self.translation.types.add_local_id(name.into(), local_type)
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
        let mut tx = mpsc::UnboundedSender::clone(&self.system_tx);
        tx.unbounded_send(message)
            .map_err(|e| Error::OtherMessage(e.to_string()))?;
        Ok(())
    }

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
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
    }
}

// impl Future for EndpointIp {
//     type Item = ();
//     type Error = Error;
//     fn poll(&mut self, dispatcher: &TypeDispatcher) -> Poll<Self::Item, Self::Error> {
//         self.poll_endpoint(dispatcher)
//     }
// }

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
                    let mut channel = ep.reliable_channel.lock().unwrap();
                    let _ = channel
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
