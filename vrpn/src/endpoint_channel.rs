// Copyright (c) 2018 Tokio Contributors
// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: MIT
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>, based in part on
// https://github.com/tokio-rs/tokio/blob/24d99c029eff5d5b82aff567f1ad5ede8a8c2576/examples/chat.rs

use bytes::BytesMut;
use crate::{
    base::message::{GenericMessage, Message},
    buffer::{
        buffer,
        message::{make_message_body_generic, MessageSize},
        unbuffer, Buffer, Output, Unbuffer,
    },
    codec::{self, FramedMessageCodec},
    connection::{
        typedispatcher::HandlerResult, Endpoint, TranslationTable, TranslationTableError,
        TranslationTableResult,
    },
    prelude::*,
};
use futures::{sync::mpsc, StartSend};

use tokio::{
    codec::{Decoder, Encoder, Framed},
    io,
    net::{TcpStream, UdpFramed, UdpSocket},
    prelude::*,
};

quick_error!{
    #[derive(Debug)]
    pub enum EndpointError {
        IoError(err: io::Error) {
            from()
            cause(err)
            display("IO error: {}", err)
        }
        BufferError(err: buffer::Error) {
            from()
            cause(err)
            display("buffer error: {}", err)
        }
        UnbufferError(err: unbuffer::Error) {
            from()
            cause(err)
            display("unbuffer error: {}", err)
        }
        MpscSendError(err: mpsc::SendError<GenericMessage>) {
            from()
            cause(err)
            display("mpsc send error: {}", err)
        }
        // NoneError(err: std::option::NoneError) {
        //     from()
        //     cause(err)
        //     display("none error: {}", err)
        // }
    }
}

type EpSinkItem = GenericMessage;
type EpSinkError = <FramedMessageCodec as Encoder>::Error;
type EpStreamItem = EpSinkItem;
type EpStreamError = <FramedMessageCodec as Decoder>::Error;

pub(crate) enum EndpointDisposition {
    StillActive,
    ReadyForCleanup,
}
pub type ReceiveFuture = Box<dyn Future<Item = (), Error = EndpointError>>;
type Tx = mpsc::UnboundedSender<GenericMessage>;
type Rx = mpsc::UnboundedReceiver<GenericMessage>;
pub(crate) struct EndpointChannel<T> {
    tx: stream::SplitSink<T>,
    rx: stream::SplitStream<T>,

    buf_size: usize,

    // /// The "internal" end of the mpsc channel for sending messages out the endpoint.
    // out_rx: Rx,

    // /// The "external" end of the mpsc channel for sending messages out the endpoint.
    // out_tx: Option<Tx>,
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
    pub(crate) fn new(framed_stream: T, buf_size: usize) -> EndpointChannel<T> {
        // ugh order of tx and rx is different between AsyncWrite::split() and Framed<>::split()
        let (tx, rx) = framed_stream.split();
        // let (out_tx, out_rx) = mpsc::unbounded();
        let (in_tx, in_rx) = mpsc::unbounded();
        EndpointChannel {
            tx,
            buf_size,
            rx,
            // out_rx,
            // out_tx: Some(out_tx),
            in_tx,
            in_rx,
        }
    }
    /// Buffer a message.
    ///
    /// This serializes a message to an internal buffer. Calls to `poll_flush` will
    /// attempt to flush this buffer to the socket.
    fn buffer<U: Buffer>(&mut self, message: Message<U>) -> StartSend<GenericMessage, EpSinkError> {
        let message = make_message_body_generic(message)?;
        self.tx.start_send(message)?;
        Ok(AsyncSink::Ready)
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), EpSinkError> {
        // const MESSAGES_PER_TICK: usize = 10;

        // // Receive all messages from peers.
        // for i in 0..MESSAGES_PER_TICK {
        //     // Polling an `UnboundedReceiver` cannot fail, so `unwrap` here is
        //     // safe.
        //     match self.out_rx.poll().unwrap() {
        //         Async::Ready(Some(message)) => {
        //             // Buffer the message. Once all messages are buffered, they will
        //             // be flushed to the socket (right below).
        //             self.buffer(message)?;

        //             // If this is the last iteration, the loop will break even
        //             // though there could still be lines to read. Because we did
        //             // not reach `Async::NotReady`, we have to notify ourselves
        //             // in order to tell the executor to schedule the task again.
        //             if i + 1 == MESSAGES_PER_TICK {
        //                 task::current().notify();
        //             }
        //         }
        //         _ => break,
        //     }
        // }

        // Flush the write buffer to the socket
        self.tx.poll_complete()
    }

    fn receive(&mut self) -> Poll<(), EndpointError> {
        loop {
            let poll = self.rx.poll()?;
            match poll {
                Async::Ready(msg) => {
                    println!("receive got message {:?}", msg);
                    match msg {
                        Some(msg) => self.in_tx.unbounded_send(msg).unwrap(),
                        None => return Ok(Async::Ready(())),
                    }
                }
                Async::NotReady => return Ok(Async::NotReady),
            }
            // if let Some(msg) = try_ready!(self.rx.poll()) {
            //     self.in_tx.unbounded_send(msg).unwrap();
            // } else {
            //     break;
            // }
        }
        // Other end has disconnected
        return Ok(Async::Ready(()));
    }

    /// Method for polling the MPSC channel that decoded generic messages are placed in.
    fn poll_receive(&mut self) -> Poll<Option<GenericMessage>, EndpointError> {
        // treat errors like a closed connection
        let poll = self.in_rx.poll();
        poll.or(Ok(Async::Ready(None)))
    }

    /// Async::Ready(()) means the channel was closed.
    /// Async::NotReady is returned otherwise.
    pub(crate) fn poll_channel<F>(&mut self, message_handler: F) -> Poll<(), EndpointError>
    where
        F: FnMut(GenericMessage) -> Result<(), EndpointError>,
    {
        let mut message_handler = message_handler;
        let _ = self.poll_flush()?;

        let _ = self.receive()?;
        const MAX_PER_TICK: usize = 10;
        for i in 0..MAX_PER_TICK {
            match try_ready!(self.poll_receive()) {
                Some(msg) => {
                    println!("poll_channel: received message {:?}", msg);
                    message_handler(msg)?;
                }
                None => {
                    println!("poll_channel: received None");
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
