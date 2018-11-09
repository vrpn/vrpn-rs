// Copyright (c) 2018 Tokio Contributors
// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: MIT
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>, based in part on
// https://github.com/tokio-rs/tokio/blob/24d99c029eff5d5b82aff567f1ad5ede8a8c2576/examples/chat.rs

use bytes::BytesMut;
use crate::{
    base::message::{GenericMessage, Message, SequencedGenericMessage},
    buffer::{buffer, unbuffer},
    codec::FramedMessageCodec,
    error::Error,
    prelude::*,
};
use futures::{sync::mpsc, StartSend};

use tokio::{
    codec::{Decoder, Encoder},
    io,
    prelude::*,
};

pub(crate) type EpSinkItem = SequencedGenericMessage;
pub(crate) type EpSinkError = <FramedMessageCodec as Encoder>::Error;
pub(crate) type EpStreamItem = EpSinkItem;
pub(crate) type EpStreamError = <FramedMessageCodec as Decoder>::Error;
type Tx = mpsc::UnboundedSender<SequencedGenericMessage>;
type Rx = mpsc::UnboundedReceiver<SequencedGenericMessage>;

#[derive(Debug)]
pub(crate) struct EndpointChannel<T> {
    tx: stream::SplitSink<T>,
    rx: stream::SplitStream<T>,

    /// The "internal" end of the mpsc channel for returning received messages from the endpoint.
    in_tx: Tx,

    /// The "external" end of the mpsc channel for returning received messages from the endpoint.
    in_rx: Rx,
}

impl<T> EndpointChannel<T>
where
    T: Sink<SinkItem = EpSinkItem, SinkError = EpSinkError>
        + Stream<Item = EpStreamItem, Error = EpStreamError>,
{
    pub(crate) fn new(framed_stream: T) -> EndpointChannel<T> {
        // ugh order of tx and rx is different between AsyncWrite::split() and Framed<>::split()
        let (tx, rx) = framed_stream.split();
        let (in_tx, in_rx) = mpsc::unbounded();
        EndpointChannel {
            tx,
            rx,
            in_tx,
            in_rx,
        }
    }
    /// Buffer a message.
    ///
    /// This serializes a message to an internal buffer. Calls to `poll_flush` will
    /// attempt to flush this buffer to the socket.
    pub(crate) fn buffer(
        &mut self,
        message: SequencedGenericMessage,
    ) -> StartSend<SequencedGenericMessage, EpSinkError> {
        self.tx.start_send(message)?;
        Ok(AsyncSink::Ready)
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), EpSinkError> {
        self.tx.poll_complete()
    }

    /// Method for polling the MPSC channel that decoded generic messages are placed in.
    fn poll_receive(&mut self) -> Poll<Option<GenericMessage>, Error> {
        // treat errors like a closed connection
        self.rx
            .poll()
            .map(|a| a.map(|o| o.map(|msg| Message::from(msg))))
            .map_err(|e| Error::from(e))
    }

    /// Async::Ready(()) means the channel was closed.
    /// Async::NotReady is returned otherwise.
    pub(crate) fn process_send_receive<F>(&mut self, message_handler: F) -> Poll<(), Error>
    where
        F: FnMut(GenericMessage) -> Result<(), Error>,
    {
        let mut message_handler = message_handler;
        let _ = self.poll_flush()?;

        // let _ = self.receive()?;
        const MAX_PER_TICK: usize = 10;
        for i in 0..MAX_PER_TICK {
            match try_ready!(self.poll_receive()) {
                Some(msg) => {
                    eprintln!("poll_channel: received message {:?}", msg);
                    message_handler(GenericMessage::from(msg))?;
                }
                None => {
                    eprintln!("poll_channel: received None");
                    return Ok(Async::Ready(()));
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

        // OK to do because we either got not ready from self.poll_flush(),
        // self.receive(), or we've hit the limit (and have notified ourselves again accordingly)
        Ok(Async::NotReady)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn make_endpoint_channel() {
        use crate::{codec::apply_message_framing, connect::connect_tcp};
        let addr = "127.0.0.1:3883".parse().unwrap();
        let _ = connect_tcp(addr)
            .and_then(|stream| {
                let mut chan = EndpointChannel::new(apply_message_framing(stream));
                // future::poll_fn(move || ep.poll())
                // .map_err(|e| {
                //     eprintln!("{}", e);
                //     panic!()
                // })
                for _i in 0..4 {
                    let _ = chan
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
