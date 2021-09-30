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
use futures::future::{BoxFuture, Fuse, FusedFuture, LocalBoxFuture};
use futures::io::{BufReader, BufWriter};
use futures::{pin_mut, ready, AsyncWriteExt, Future, FutureExt};
use futures::{AsyncRead, AsyncWrite, Sink, SinkExt, Stream, StreamExt};
use std::fmt::Debug;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use super::AsyncReadMessagesExt;
use pin_project_lite::pin_project;

#[derive(Debug)]
pub(crate) struct EndpointRx<T> {
    stream: Pin<Box<T>>,
    error: Option<VrpnError>,
}

impl<T> EndpointRx<T> where T: Stream<Item = SequencedGenericMessage> {}

impl<U: AsyncRead + Unpin> EndpointRx<MessageStream<U>> {
    pub(crate) fn from_reader(reader: U) -> Arc<Mutex<EndpointRx<MessageStream<U>>>> {
        Arc::new(Mutex::new(EndpointRx {
            stream: Box::pin(AsyncReadMessagesExt::messages(reader)),
            error: None,
        }))
    }
}

impl<T: Stream<Item = Result<SequencedGenericMessage>>> Stream for EndpointRx<T> {
    type Item = GenericMessage;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.error.is_some() {
            return Poll::Ready(None);
        }
        match ready!(self.stream.as_mut().poll_next(cx)) {
            Some(Err(e)) => {
                self.error = Some(e);
                Poll::Ready(None)
            }
            Some(Ok(sgm)) => Poll::Ready(Some(sgm.into_inner())),
            None => Poll::Ready(None),
        }
    }
}

#[derive(Debug)]
pub(crate) enum EndpointStatus {
    Open,
    Closed,
    ClosedError(VrpnError),
}

impl EndpointStatus {
    pub(crate) fn is_error(&self) -> bool {
        match self {
            EndpointStatus::Open => false,
            EndpointStatus::Closed => false,
            EndpointStatus::ClosedError(_) => true,
        }
    }
    pub(crate) fn is_closed(&self) -> bool {
        match self {
            EndpointStatus::Open => false,
            EndpointStatus::Closed => true,
            EndpointStatus::ClosedError(_) => true,
        }
    }
    pub(crate) fn from_closed(closed: bool) -> EndpointStatus {
        match closed {
            true => EndpointStatus::Closed,
            false => EndpointStatus::Open,
        }
    }
    // pub(crate) fn accumulate_closed(&mut self, other: EndpointStatus) {
    //     let max = self.max(&mut other);
    //     self = max;
    //     // if self == EndpointStatus::Closed {return;}
    //     // if other == EndpointStatus::Closed {
    //     //     self = EndpointStatus
    //     // }
    // }
}
pub(crate) fn merge_status(a: EndpointStatus, b: EndpointStatus) -> EndpointStatus {
    if a.is_error() {
        a
    } else if b.is_error() {
        b
    } else if a.is_closed() || b.is_closed() {
        EndpointStatus::Closed
    } else {
        EndpointStatus::Open
    }
}

pub(crate) trait ToEndpointStatus {
    fn to_endpoint_status(self) -> EndpointStatus;
}

impl ToEndpointStatus for std::task::Poll<std::result::Result<(), VrpnError>> {
    fn to_endpoint_status(self) -> EndpointStatus {
        match self {
            Poll::Ready(Ok(())) => EndpointStatus::Closed,
            Poll::Ready(Err(e)) => EndpointStatus::ClosedError(e),
            Poll::Pending => EndpointStatus::Open,
        }
    }
}

impl From<EndpointStatus> for std::task::Poll<std::result::Result<(), VrpnError>> {
    fn from(val: EndpointStatus) -> Self {
        match val {
            EndpointStatus::Open => Poll::Pending,
            EndpointStatus::Closed => Poll::Ready(Ok(())),
            EndpointStatus::ClosedError(e) => Poll::Ready(Err(e)),
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
