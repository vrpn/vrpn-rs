// Copyright (c) 2018 Tokio Contributors
// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: MIT
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>, based in part on
// https://github.com/tokio-rs/tokio/blob/24d99c029eff5d5b82aff567f1ad5ede8a8c2576/examples/chat.rs

use crate::{
    vrpn_tokio::endpoint_ip::EndpointIp, Endpoint, Error, GenericMessage, LocalId, Message,
    MessageHeader, RemoteId, SequenceNumber, SequencedGenericMessage, TypeDispatcher,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::prelude::*;

#[derive(Debug)]
pub(crate) struct EndpointChannel<T> {
    tx: stream::SplitSink<T>,
    rx: stream::SplitStream<T>,
    seq: AtomicUsize,
}

impl<T> EndpointChannel<T>
where
    T: Sink<SinkItem = SequencedGenericMessage, SinkError = Error>
        + Stream<Item = SequencedGenericMessage, Error = Error>,
{
    pub(crate) fn new(framed_stream: T) -> Arc<Mutex<EndpointChannel<T>>> {
        // ugh order of tx and rx is different between AsyncWrite::split() and Framed<>::split()
        let (tx, rx) = framed_stream.split();
        Arc::new(Mutex::new(EndpointChannel {
            tx,
            rx,
            seq: AtomicUsize::new(0),
        }))
    }
}

impl<T> Stream for EndpointChannel<T>
where
    T: Sink<SinkItem = SequencedGenericMessage, SinkError = Error>
        + Stream<Item = SequencedGenericMessage, Error = Error>,
{
    type Item = GenericMessage;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // treat errors like a closed connection
        self.rx
            .poll()
            // these nested maps are to get all the way inside the Ok(Async::Ready(Some(msg)))
            .map(|a| a.map(|o| o.map(|msg| Message::from(msg))))
    }
}

impl<T> Sink for EndpointChannel<T>
where
    T: Sink<SinkItem = SequencedGenericMessage, SinkError = Error>
        + Stream<Item = SequencedGenericMessage, Error = Error>,
{
    type SinkItem = GenericMessage;
    type SinkError = Error;
    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        match self
            .tx
            .start_send(item.into_sequenced_message(SequenceNumber(seq as u32)))?
        {
            AsyncSink::Ready => Ok(AsyncSink::Ready),

            // Unwrap the message again if not ready.
            AsyncSink::NotReady(msg) => Ok(AsyncSink::NotReady(GenericMessage::from(msg))),
        }
    }
    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        self.tx.poll_complete()
    }
}

/// Given a stream of GenericMessage, poll the stream and dispatch received messages.
pub(crate) fn poll_and_dispatch<T>(
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
                    if let Some(LocalId(new_type)) =
                        endpoint.map_to_local_id(RemoteId(msg.header.message_type))
                    {
                        if let Some(LocalId(new_sender)) =
                            endpoint.map_to_local_id(RemoteId(msg.header.sender))
                        {
                            let msg = Message::from_header_and_body(
                                MessageHeader::new(
                                    Some(msg.header.time.clone()),
                                    new_type,
                                    new_sender,
                                ),
                                msg.body,
                            );
                            dispatcher.call(&msg)?;
                        } else {
                            eprintln!("Could not map sender to local");
                        }
                    } else {
                        eprintln!("Could not map type to local");
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vrpn_tokio::{apply_message_framing, connect_tcp};
    #[test]
    fn make_endpoint_channel() {
        let addr = "127.0.0.1:3883".parse().unwrap();
        let _ = connect_tcp(addr)
            .and_then(|stream| {
                let chan = EndpointChannel::new(apply_message_framing(stream));
                for _i in 0..4 {
                    let _ = chan.lock().unwrap().poll().unwrap().map(|msg| {
                        eprintln!("Received message {:?}", msg);
                        msg
                    });
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }
}
