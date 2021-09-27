// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use std::borrow::BorrowMut;

use crate::{
    buffer_unbuffer::{peek_u32, BufferTo, BufferUnbufferError, BytesMutExtras, UnbufferFrom},
    data_types::{
        cookie::check_ver_nonfile_compatible, CookieData, MessageSize, SequencedGenericMessage,
        TypedMessage,
    },
    handler::{HandlerCode, TypedHandler},
    tracker::PoseReport,
    Result, TypeDispatcher, VrpnError,
};
use async_std::{
    io::BufReader,
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use bytes::{Buf, BytesMut};
use futures::io::Read;
use futures::{ready, AsyncBufReadExt, AsyncRead, AsyncReadExt, FutureExt};
use pin_project_lite::pin_project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageStreamState {
    // NeedRead,
    Reading,
    Parsing,
    Error,
}
pin_project! {
    pub struct MessageStream<R> {
        #[pin]
        stream: R,
        state: MessageStreamState,
        // mini_buf: Box<[u8; 1024]>,
        mini_buf: [u8; 1024],
        buf: BytesMut,
        // #[pin]
        // read: S,
    }
}

impl<'a, R: AsyncReadExt + Unpin> MessageStream<R> {
    pub fn new(stream: R) -> MessageStream<R> {
        MessageStream {
            stream,
            state: MessageStreamState::Reading,
            mini_buf: [0u8; 1024],
            buf: BytesMut::with_capacity(2048),
            // read: AsyncReadExt::read(&mut stream, mini_buf.as_mut()),
            // mini_buf: ,
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
            println!("State: {:?}", state);
            match state {
                // MessageStreamState::NeedRead => {
                //     pinned.read = AsyncReadExt::read(&mut self.stream, pinned.mini_buf.as_mut());
                //     *state = MessageStreamState::Reading;
                // }
                MessageStreamState::Reading => {
                    match ready!(pinned
                        .stream
                        .as_mut()
                        .poll_read(cx, pinned.mini_buf.borrow_mut()))
                    {
                        Ok(n) => {
                            println!("Read {} bytes from stream", n);
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
                            println!(
                                "consumed {} bytes, {} remain in buffer",
                                consumed,
                                pinned.buf.remaining()
                            );
                            println!("{:?}", pinned.buf);
                            cx.waker().wake_by_ref();
                            return task::Poll::Ready(Some(Ok(sgm)));
                        }
                        Err(BufferUnbufferError::NeedMoreData(requirement)) => {
                            println!("need more data: {} bytes", requirement);
                            *state = MessageStreamState::Reading;
                        }
                        Err(e) => {
                            *state = MessageStreamState::Error;
                            return task::Poll::Ready(Some(Err(e.into())));
                        }
                    }
                }
                MessageStreamState::Error => {
                    return task::Poll::Ready(None);
                }
            }
        }
    }
}

// pub struct MessageStream<R> {
//     inner: MessageStream<R>,
// }

// impl<R: AsyncRead + Unpin> MessageStream<R> {
//     pub fn new(reader: R) -> MessageStream<R> {
//         MessageStream {
//             inner: MessageStream::new(reader),
//         }
//     }
// }

// impl<R: AsyncRead + Unpin> Stream for MessageStream<R> {
//     type Item = Result<SequencedGenericMessage>;

//     fn poll_next(
//         self: std::pin::Pin<&mut Self>,
//         cx: &mut task::Context<'_>,
//     ) -> task::Poll<Option<Self::Item>> {
//         self.inner.project().poll_next(cx)
//     }
// }

pub trait AsyncReadMessagesExt: AsyncRead + Unpin + Sized {
    fn messages(self) -> MessageStream<Self>;
}

impl<T: AsyncRead + Unpin> AsyncReadMessagesExt for T {
    /// Adapt stream to parse messages
    fn messages(self) -> MessageStream<Self> {
        MessageStream::new(self)
    }
}

// fn poll_message_internal<'a, R: AsyncReadExt + Unpin>(
//     stream: &'a mut R,
//     buf: &mut BytesMut,
//     // mini_buf: &'a mut Box<[u8; 1024]>,
//     state: &mut MessageStreamState<R>,
//     cx: &mut task::Context<'a>,
// ) -> task::Poll<Result<Option<SequencedGenericMessage>>> {
//     loop {
//         match state {
//             MessageStreamState::NeedRead => {
//                 let mini_buf = Box::new([0u8; 1024]);
//                 let state_projection = state.project();

//                 *state = MessageStreamState::Reading {
//                     AsyncReadExt::read(stream, mini_buf.as_mut())
//                 };
//             }
//             MessageStreamState::Reading { mini_buf, read }=> match ready!(read.poll_unpin(cx)) {
//                 Ok(n) => {
//                     println!("Read {} bytes from stream", n);
//                     buf.extend_from_slice(&mini_buf[..n]);
//                     *state = MessageStreamState::Parsing;
//                 }
//                 Err(e) => {
//                     *state = MessageStreamState::Error;
//                     return Poll::Ready(Err(e.into()));
//                 }
//             },
//             MessageStreamState::Parsing => {
//                 let mut existing_bytes = std::io::Cursor::new(&*buf);

//                 match SequencedGenericMessage::try_read_from_buf(&mut existing_bytes) {
//                     Ok(sgm) => {
//                         // consume the bytes from the original buffer.
//                         let consumed = buf.remaining() - existing_bytes.remaining();
//                         buf.advance(consumed);
//                         println!(
//                             "consumed {} bytes, {} remain in buffer",
//                             consumed,
//                             buf.remaining()
//                         );
//                         println!("{:?}", buf);
//                         return Poll::Ready(Ok(Some(sgm)));
//                     }
//                     Err(BufferUnbufferError::NeedMoreData(requirement)) => {
//                         println!("need more data: {} bytes", requirement);
//                         *state = MessageStreamState::NeedRead;
//                     }
//                     Err(e) => {
//                         *state = MessageStreamState::Error(Box::new(e));
//                         return Poll::Ready(Err(e.into()));
//                     }
//                 }
//             }
//             MessageStreamState::Error(_) => {
//                 return Poll::Ready(Ok(None));
//             }
//         }
//     }
// }
