// Copyright (c) 2018 Tokio Contributors
// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: MIT
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>, based in part on
// https://github.com/tokio-rs/tokio/blob/24d99c029eff5d5b82aff567f1ad5ede8a8c2576/examples/chat.rs

use super::{
    base::{
        message::{Description, GenericMessage, Message},
        types::*,
    },
    buffer::{buffer, message::MessageSize, unbuffer, Buffer, Output, Unbuffer},
    codec::{self, FramedMessageCodec},
    connection::{
        typedispatcher::HandlerResult, Endpoint, TranslationTable, TranslationTableError,
        TranslationTableResult,
    },
    prelude::*,
};
use bytes::BytesMut;
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
}
}
pub(crate) enum EndpointDisposition {
    StillActive,
    ReadyForCleanup,
}

pub(crate) struct EndpointChannel<T>
where
    T: AsyncRead + AsyncWrite,
{
    socket: T,
    buf_size: usize,
    wr: BytesMut,
    rd: BytesMut,
}

impl<T> EndpointChannel<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub(crate) fn new(socket: T, buf_size: usize) -> EndpointChannel<T> {
        // ugh order of tx and rx is different between AsyncWrite::split() and Framed<>::split()
        EndpointChannel {
            socket,
            buf_size,
            wr: BytesMut::new(),
            rd: BytesMut::with_capacity(buf_size),
        }
    }
    /// Buffer a message.
    ///
    /// This serializes a message to an internal buffer. Calls to `poll_flush` will
    /// attempt to flush this buffer to the socket.
    pub(crate) fn buffer<U: Buffer>(&mut self, message: &Message<U>) -> buffer::Result {
        // Ensure the buffer has capacity.
        self.wr.reserve(message.required_buffer_size());

        message.buffer_ref(&mut self.wr)
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // As long as there is buffered data to write, try to write it.
        while !self.wr.is_empty() {
            // Try to write some bytes to the socket
            let n = try_ready!(self.socket.poll_write(&self.wr));

            // As long as the wr is not empty, a successful write should
            // never write 0 bytes.
            assert!(n > 0);

            // This discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }

    /// Read data from the socket.
    ///
    /// This only returns `Ready` when the socket has closed.
    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            // Ensure the read buffer has capacity.
            //
            // This might result in an internal allocation.
            self.rd.reserve(self.buf_size);

            // Read data into the buffer.
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl<T> Stream for EndpointChannel<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Item = GenericMessage;
    type Error = EndpointError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // First, read any new data that might have been received off the socket
        let sock_closed = self.fill_read_buf()?.is_ready();

        // Now, try parsing.
        let initial_len = self.rd.len();
        let mut temp_buf = BytesMut::clone(&self.rd).freeze();
        let combined_size = u32::unbuffer_ref(&mut temp_buf)
            .map_exactly_err_to_at_least()?
            .data() as usize;
        let size = MessageSize::from_unpadded_message_size(combined_size);
        let not_enough = if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        };
        if initial_len < size.padded_message_size() {
            return not_enough;
        }
        let mut temp_buf = BytesMut::clone(&self.rd).freeze();
        match GenericMessage::unbuffer_ref(&mut temp_buf) {
            Ok(Output(v)) => {
                self.rd.advance(initial_len - temp_buf.len());
                Ok(Async::Ready(Some(v)))
            }
            Err(unbuffer::Error::NeedMoreData(_)) => not_enough,
            Err(e) => Err(Self::Error::from(e)),
        }
    }
}

pub(crate) fn poll_channel<T>(
    ch: &mut EndpointChannel<T>,
) -> Poll<Option<GenericMessage>, EndpointError>
where
    T: AsyncRead + AsyncWrite,
{
    let _ = ch.poll_flush()?;

    while let Async::Ready(msg) = ch.poll()? {
        return Ok(Async::Ready(msg));
    }
    // As always, it is important to not just return `NotReady` without
    // ensuring an inner future also returned `NotReady`.
    //
    // We know we got a `NotReady` from either `self.rx` or `self.lines`, so
    // the contract is respected.
    Ok(Async::NotReady)
}
