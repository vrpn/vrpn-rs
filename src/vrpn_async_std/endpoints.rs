// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::MessageStream;
use crate::buffer_unbuffer::BytesMutExtras;
use crate::data_types::id_types::SequenceNumber;
use crate::data_types::{GenericMessage, Message, SequencedGenericMessage};
use crate::error::to_other_error;
use crate::{endpoint::*, Result, TranslationTables, TypeDispatcher, VrpnError};
use bytes::{Bytes, BytesMut};
use futures::channel::mpsc;
use futures::{ready, AsyncWriteExt};
use futures::{AsyncRead, AsyncWrite, Sink, SinkExt, Stream, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use super::AsyncReadMessagesExt;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    pub(crate) struct EndpointTx<T> {
        #[pin]
        writer: T,
        now_sending: Option<Bytes>,
        seq: AtomicUsize,
        channel_rx: mpsc::UnboundedReceiver<SequencedGenericMessage>,
        channel_tx: mpsc::UnboundedSender<SequencedGenericMessage>,
    }
}

impl<T: AsyncWrite + Unpin> EndpointTx<T> {
    pub(crate) fn new(writer: T) -> Arc<Mutex<EndpointTx<T>>> {
        let (channel_tx, channel_rx) = mpsc::unbounded();
        Arc::new(Mutex::new(EndpointTx {
            writer,
            now_sending: None,
            seq: AtomicUsize::new(0),
            channel_rx,
            channel_tx,
        }))
    }

    pub async fn send_generic_message(&mut self, msg: GenericMessage) -> Result<()> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);

        let msg = msg.into_sequenced_message(SequenceNumber(seq as u32));
        let buf = msg.try_into_buf()?;
        self.writer.write_all(&buf[..]).await?;
        Ok(())
    }
}

// impl<T: AsyncWrite> Sink<GenericMessage> for EndpointTx<T> {
//     type Error = VrpnError;

//     fn poll_ready(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> Poll<std::result::Result<(), Self::Error>> {
//         ready!(self.channel_tx.poll_ready_unpin(cx).map_err(to_other_error)?);
//         if let Some(sending) = self.now_sending {

//             self.project().writer.poll_write(cx, buf)
//         }
//     }

//     fn start_send(
//         self: std::pin::Pin<&mut Self>,
//         item: GenericMessage,
//     ) -> std::result::Result<(), Self::Error> {
//         todo!()
//     }

//     fn poll_flush(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> Poll<std::result::Result<(), Self::Error>> {
//         todo!()
//     }

//     fn poll_close(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> Poll<std::result::Result<(), Self::Error>> {
//         todo!()
//     }
// }

#[derive(Debug)]
pub(crate) struct EndpointRx<T> {
    stream: T,
    error: Option<VrpnError>,
}

impl<T> EndpointRx<T> where T: Stream<Item = SequencedGenericMessage> {}
impl<U: AsyncRead + Unpin> EndpointRx<MessageStream<U>> {
    pub(crate) fn from_reader(reader: U) -> Arc<Mutex<EndpointRx<MessageStream<U>>>> {
        Arc::new(Mutex::new(EndpointRx {
            stream: AsyncReadMessagesExt::messages(reader),
            error: None,
        }))
    }
}

impl<T: Stream<Item = Result<SequencedGenericMessage>>> Stream for EndpointRx<T> {
    type Item = GenericMessage;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.error.is_some() {
            return Poll::Ready(None);
        }
        match ready!(Box::pin(self.stream).poll_next_unpin(cx)) {
            Some(Err(e)) => {
                self.error = Some(e);
                Poll::Ready(None)
            }
            Some(Ok(sgm)) => Poll::Ready(Some(sgm.into_inner())),
            None => Poll::Ready(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EndpointStatus {
    Open = 0,
    Closed = 1,
}

impl EndpointStatus {
    pub(crate) fn from_closed(closed: bool) -> EndpointStatus {
        match closed {
            true => EndpointStatus::Closed,
            false => EndpointStatus::Open,
        }
    }
    pub(crate) fn accumulate_closed(&mut self, other: EndpointStatus) {
        let max  = self.max(&mut other);
        self = max;
        // if self == EndpointStatus::Closed {return;}
        // if other == EndpointStatus::Closed {
        //     self = EndpointStatus
        // }
    }
}

pub(crate) trait ToEndpointStatus {
    fn to_endpoint_status(self) -> EndpointStatus;
}

impl<T> ToEndpointStatus for std::task::Poll<T> {
    fn to_endpoint_status(self) -> EndpointStatus {
        match self {
            Poll::Ready(_) => EndpointStatus::Closed,
            Poll::Pending => EndpointStatus::Open,
        }
    }
}

/// Given a stream of GenericMessage, poll the stream and dispatch received messages.
///
/// Is only ready when the stream is closed.
pub(crate) fn poll_and_dispatch<T, U>(
    endpoint: &mut T,
    stream: &mut U,
    dispatcher: &mut TypeDispatcher,
    cx: &mut Context<'_>,
) -> Poll<std::result::Result<(), VrpnError>>
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
                if msg.is_system_message() {
                    endpoint.send_system_change(parse_system_message(msg)?);
                } else {
                    dispatcher.call(&msg)?;
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
