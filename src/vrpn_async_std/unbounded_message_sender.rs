// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    data_types::{id_types::SequenceNumber, GenericMessage},
    error::to_other_error,
    Result, VrpnError,
};
use futures::{
    channel::mpsc, future::FusedFuture, io::BufWriter, stream::futures_unordered, AsyncWrite,
    AsyncWriteExt, Future, FutureExt, StreamExt,
};
use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};

/// The actual async function underlying UnboundedMessageSender
async fn sender<T: AsyncWrite>(
    stream: T,
    channel_rx: mpsc::UnboundedReceiver<GenericMessage>,
) -> Result<()> {
    let mut seq: u32 = 0;
    let mut channel_rx = channel_rx;
    let mut stream = Box::pin(BufWriter::new(stream));
    while let Some(msg) = channel_rx.next().await {
        seq += 1;
        let msg = msg.into_sequenced_message(SequenceNumber(seq));
        let buf = msg.try_into_buf()?;
        stream.write_all(&buf).await?;
    }
    Ok(())
}

type FusedBoxFuture<'a, T> = Pin<Box<dyn FusedFuture<Output = T> + Send + 'a>>;

/// A structure that lets you send messages to some stream just like an unbounded channel
pub(crate) struct UnboundedMessageSender {
    channel_tx: mpsc::UnboundedSender<GenericMessage>,
    send_future: FusedBoxFuture<'static, Result<()>>,
}

impl UnboundedMessageSender {
    /// Create a future that pumps transmission of sequenced messages to an AsyncWrite implementation.
    pub(crate) fn new<T: 'static + AsyncWrite + Send>(
        writer: T,
    ) -> Pin<Box<UnboundedMessageSender>> {
        let (channel_tx, channel_rx) = mpsc::unbounded();
        Box::pin(UnboundedMessageSender {
            channel_tx,
            send_future: Box::pin(sender(writer, channel_rx).fuse()),
        })
    }
}

impl UnboundedMessageSender {
    /// Queues a message to be sequenced and sent.
    pub(crate) fn unbounded_send(self: Pin<&mut Self>, msg: GenericMessage) -> Result<()> {
        if self.is_terminated() {
            return Err(VrpnError::EndpointClosed);
        }
        self.channel_tx
            .unbounded_send(msg)
            .map_err(to_other_error)?;
        Ok(())
    }

    /// Closes the channel feeding this this sender
    pub(crate) fn close(&mut self) {
        if !self.is_terminated() {
            self.channel_tx.close_channel()
        }
    }
}

impl Debug for UnboundedMessageSender {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("UnboundedMessageSender")
            .field("channel_tx", &self.channel_tx)
            .field("send_future", &!self.send_future.is_terminated())
            .finish()
    }
}

impl Future for UnboundedMessageSender {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.send_future.as_mut().poll(cx)
    }
}

impl Unpin for UnboundedMessageSender {}

impl FusedFuture for UnboundedMessageSender {
    fn is_terminated(&self) -> bool {
        self.send_future.is_terminated() || self.channel_tx.is_closed()
    }
}
