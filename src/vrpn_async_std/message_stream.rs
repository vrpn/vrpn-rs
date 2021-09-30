// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::borrow::BorrowMut;

use crate::{buffer_unbuffer::BufferUnbufferError, data_types::SequencedGenericMessage, Result};
use async_std::{prelude::*, task};
use bytes::{Buf, BytesMut};

use futures::{ready, AsyncRead, AsyncReadExt};
use pin_project_lite::pin_project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageStreamState {
    // NeedRead,
    Reading,
    Parsing,
    Error,
}
pin_project! {
    #[derive(Debug)]
    pub struct MessageStream<R> {
        #[pin]
        stream: R,
        state: MessageStreamState,
        mini_buf: [u8; 1024],
        buf: BytesMut,
    }
}

impl<'a, R: AsyncReadExt + Unpin> MessageStream<R> {
    pub fn new(stream: R) -> MessageStream<R> {
        MessageStream {
            stream,
            state: MessageStreamState::Reading,
            mini_buf: [0u8; 1024],
            buf: BytesMut::with_capacity(2048),
        }
    }
}

impl<R> Stream for MessageStream<R>
where
    R: AsyncRead + Unpin,
{
    type Item = Result<SequencedGenericMessage>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let mut pinned = self.project();
        let state = pinned.state.borrow_mut();
        loop {
            // println!("State: {:?}", state);
            match state {
                MessageStreamState::Reading => {
                    match ready!(pinned
                        .stream
                        .as_mut()
                        .poll_read(cx, pinned.mini_buf.borrow_mut()))
                    {
                        Ok(n) => {
                            // println!("Read {} bytes from stream", n);
                            pinned.buf.extend_from_slice(&pinned.mini_buf[..n]);
                            *state = MessageStreamState::Parsing;
                        }
                        Err(e) => {
                            *state = MessageStreamState::Error;
                            return task::Poll::Ready(Some(Err(e.into())));
                        }
                    }
                }
                MessageStreamState::Parsing => {
                    let mut existing_bytes = std::io::Cursor::new(&*pinned.buf);

                    match SequencedGenericMessage::try_read_from_buf(&mut existing_bytes) {
                        Ok(sgm) => {
                            // consume the bytes from the original buffer.
                            let consumed = pinned.buf.remaining() - existing_bytes.remaining();
                            pinned.buf.advance(consumed);
                            // println!(
                            //     "consumed {} bytes, {} remain in buffer",
                            //     consumed,
                            //     pinned.buf.remaining()
                            // );
                            // println!("{:?}", pinned.buf);

                            // Queue an immediate wakeup since the buf may contain more.
                            cx.waker().wake_by_ref();
                            return task::Poll::Ready(Some(Ok(sgm)));
                        }
                        Err(BufferUnbufferError::NeedMoreData(_requirement)) => {
                            // println!("need more data: {} bytes", _requirement);
                            *state = MessageStreamState::Reading;
                        }
                        Err(e) => {
                            *state = MessageStreamState::Error;
                            return task::Poll::Ready(Some(Err(e.into())));
                        }
                    }
                }
                MessageStreamState::Error => {
                    // once in this state we never escape
                    return task::Poll::Ready(None);
                }
            }
        }
    }
}

pub trait AsyncReadMessagesExt: AsyncRead + Unpin + Sized {
    fn messages(self) -> MessageStream<Self>;
}

impl<T: AsyncRead + Unpin> AsyncReadMessagesExt for T {
    /// Adapt stream to parse messages
    fn messages(self) -> MessageStream<Self> {
        MessageStream::new(self)
    }
}
