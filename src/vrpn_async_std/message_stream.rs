// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::{
    buffer_unbuffer::{peek_u32, BufferTo, BytesMutExtras, UnbufferFrom},
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
use bytes::BytesMut;
use futures::io::Read;
use futures::{ready, AsyncBufReadExt, AsyncRead, AsyncReadExt, FutureExt};
use pin_project_lite::pin_project;

// pin_project! {
//     #[derive(Debug)]
//     pub(crate) struct MessageStreamImpl<'a, R, U> {
//         #[pin]
//         inner: R,
//         read: Option<Read<'a, R>>,
//         codec:U,
//     }
// }
pin_project! {
     #[project = MessageStreamStateProj]
    pub(crate) enum MessageStreamState<T, R> {
        NeedRead,
        Reading{ #[pin] mini_buf: T, read: R},
        Parsing,
        Error,
        // Error(Box<dyn std::error::Error>),
    }
}

pub struct MessageStream<'a, R: AsyncReadExt + Unpin> {
    stream: &'a mut R,
    buf: BytesMut,
    state: MessageStreamState<[u8; 1024], R>,
}

impl<'a, T: AsyncReadExt + Unpin> MessageStream<'a, T> {
    pub fn new(stream: &'a mut T) -> MessageStream<'a, T> {
        MessageStream {
            stream,
            buf: BytesMut::with_capacity(2048),
            // mini_buf: Box::new([0u8; 1024]),
            state: MessageStreamState::NeedRead,
        }
    }
}

fn poll_message_internal<'a, R: AsyncReadExt + Unpin>(
    stream: &'a mut R,
    buf: &mut BytesMut,
    mini_buf: &'a mut Box<[u8; 1024]>,
    state: &mut MessageStreamState<'a, R>,
    cx: &mut task::Context<'_>,
) -> task::Poll<Result<Option<SequencedGenericMessage>>> {
    loop {
        match state {
            MessageStreamState::NeedRead => {
                let
                *state = MessageStreamState::Reading(AsyncReadExt::read(stream, mini_buf.as_mut()));
            }
            MessageStreamState::Reading(read) => match ready!(read.poll_unpin(cx)) {
                Ok(n) => {
                    println!("Read {} bytes from stream", n);
                    buf.extend_from_slice(&mini_buf[..n]);
                    *state = MessageStreamState::Parsing;
                }
                Err(e) => {
                    *state = MessageStreamState::Error(Box::new(e));
                    return Poll::Ready(Err(e.into()));
                }
            },
            MessageStreamState::Parsing => {
                let mut existing_bytes = std::io::Cursor::new(&*buf);

                match SequencedGenericMessage::try_read_from_buf(&mut existing_bytes) {
                    Ok(sgm) => {
                        // consume the bytes from the original buffer.
                        let consumed = buf.remaining() - existing_bytes.remaining();
                        buf.advance(consumed);
                        println!(
                            "consumed {} bytes, {} remain in buffer",
                            consumed,
                            buf.remaining()
                        );
                        println!("{:?}", buf);
                        return Poll::Ready(Ok(Some(sgm)));
                    }
                    Err(BufferUnbufferError::NeedMoreData(requirement)) => {
                        println!("need more data: {} bytes", requirement);
                        *state = MessageStreamState::NeedRead;
                    }
                    Err(e) => {
                        *state = MessageStreamState::Error(Box::new(e));
                        return Poll::Ready(Err(e.into()));
                    }
                }
            }
            MessageStreamState::Error(_) => {
                return Poll::Ready(Ok(None));
            }
        }
    }
}
