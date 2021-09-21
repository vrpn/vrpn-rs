// Copyright (c) 2018 Tokio Contributors
// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: MIT
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>, based in part on
// https://github.com/tokio-rs/tokio/blob/24d99c029eff5d5b82aff567f1ad5ede8a8c2576/examples/chat.rs

use crate::{
    Endpoint, EndpointGeneric, Error, GenericMessage, SequencedGenericMessage, TypeDispatcher,
};
use futures::StreamExt;
use futures::{
    stream::{SplitSink, SplitStream},
    Sink, Stream,
};

use std::task::Context;
use std::{
    sync::{atomic::AtomicUsize, Arc, Mutex},
    task::Poll,
};

#[derive(Debug)]
pub(crate) struct EndpointChannel<T> {
    tx: SplitSink<T, Result<SequencedGenericMessage, Error>>,
    rx: SplitStream<T>,
    seq: AtomicUsize,
}

impl<T> EndpointChannel<T>
where
    T: Sink<Result<SequencedGenericMessage, Error>> + Stream<Item = SequencedGenericMessage>,
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
    T: Sink<SequencedGenericMessage, Error = Error> + Stream<Item = SequencedGenericMessage>,
{
    type Item = Result<GenericMessage, Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // treat errors like a closed connection
        self.rx
            .poll_next_unpin(cx)
            // these nested maps are to get all the way inside the Ok(Async::Ready(Some(msg)))
            .map(|a| a.map(|a| Ok(GenericMessage::from(a))))
    }
}

// impl<T> Sink<GenericMessage> for EndpointChannel<T>
// where
//     T: Sink<SequencedGenericMessage, Error = Error> + Stream<Item = SequencedGenericMessage>,
// {
//     fn poll_ready(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         todo!()
//     }

//     fn start_send(self: Pin<&mut Self>, item: GenericMessage) -> Result<(), Self::Error> {
//         let seq = self.seq.fetch_add(1, Ordering::SeqCst);

//         let item = item.clone().into_sequenced_message(SequenceNumber(seq as u32));
//         if let Poll::Pending(msg) =  self
//             .tx
//             .start_send_unpin(item)?
//         {

//             // Unwrap the message again if not ready.
//             return Ok(Poll::NotReady(GenericMessage::from(msg)));
//         }
//     }

//     type Error = Error;

//     fn poll_flush(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         self.tx.poll_complete()
//     }

//     fn poll_close(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         todo!()
//     }
// }

/// Given a stream of GenericMessage, poll the stream and dispatch received messages.
pub(crate) fn poll_and_dispatch<T, U>(
    endpoint: &mut T,
    stream: &mut U,
    dispatcher: &mut TypeDispatcher,
    cx: &mut Context<'_>,
) -> Poll<Result<(), Error>>
where
    T: Endpoint,
    U: Stream<Item = GenericMessage> + Unpin,
{
    const MAX_PER_TICK: usize = 10;
    let mut closed = false;
    // for i in 0..MAX_PER_TICK {
    loop {
        let poll_result = stream.poll_next_unpin(cx);
        match poll_result {
            Poll::Ready(Some(msg)) => {
                let msg = endpoint.map_remote_message_to_local(msg)?;
                if let Some(nonsystem_msg) = endpoint.passthrough_nonsystem_message(msg)? {
                    dispatcher.call(&nonsystem_msg)?;
                }
            }
            Poll::Ready(None) => {
                // connection closed
                closed = true;
                break;
            }
            Poll::Pending => {
                break;
            }
        }
        // If this is the last iteration, the loop will break even
        // though there could still be messages to read. Because we did
        // not reach `Async::NotReady`, we have to notify ourselves
        // in order to tell the executor to schedule the task again.
        // if i + 1 == MAX_PER_TICK {
        //     task::current().notify();
        // }
    }
    if closed {
        eprintln!("poll_and_dispatch decided the channel was closed");
        Poll::Ready(Ok(()))
    } else {
        // eprintln!("poll_and_dispatch decided that it's not ready");
        // task::current().notify();
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        async_io::{
            apply_message_framing,
            connect::{connect, ConnectResults},
        },
        ServerInfo,
    };
    #[test]
    fn make_endpoint_channel() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let connectionResults = tokio_test::block_on(connect(server))
            .expect("should be able to create connection future");
        let ConnectResults { tcp, udp: _ } = connectionResults;
        todo!();
        // let chan = EndpointChannel::new(apply_message_framing(tcp.unwrap()));
        // for _i in 0..4 {
        //     let _ = chan.lock().unwrap().poll().unwrap().map(|msg| {
        //         eprintln!("Received message {:?}", msg);
        //         msg
        //     });
        // }
    }
}
