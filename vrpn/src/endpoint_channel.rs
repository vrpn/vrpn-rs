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
use bytes::BytesMut;
use futures::StartSend;
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
        // NoneError(err: std::option::NoneError) {
        //     from()
        //     cause(err)
        //     display("none error: {}", err)
        // }
}
}

type EpStreamItem = GenericMessage;
type EpStreamError = <FramedMessageCodec as Decoder>::Error;
type EpSinkItem = EpStreamItem;
type EpSinkError = <FramedMessageCodec as Encoder>::Error;

pub(crate) enum EndpointDisposition {
    StillActive,
    ReadyForCleanup,
}
pub type ReceiveFuture = Box<dyn Future<Item = (), Error = EpStreamError>>;

pub(crate) struct EndpointChannel<T> {
    framed_stream: T,
    buf_size: usize,
    rd: BytesMut,
    receive_future: ReceiveFuture,
}
impl<T> EndpointChannel<T>
where
    T: Sink<SinkItem = EpSinkItem, SinkError = EpSinkError>,
{
    pub(crate) fn new<U, F>(
        framed_stream: U,
        buf_size: usize,
        receiver: F,
    ) -> EndpointChannel<T> 
    where U: Stream<Item = Option<EpStreamError>, Error=EpStreamError>,
    F: FnMut(GenericMessage) -> () {
        // ugh order of tx and rx is different between AsyncWrite::split() and Framed<>::split()
        EndpointChannel {
            framed_stream,
            buf_size,
            receive_future,
            // wr: BytesMut::new(),
            rd: BytesMut::with_capacity(buf_size),
        }
    }
    /// Buffer a message.
    ///
    /// This serializes a message to an internal buffer. Calls to `poll_flush` will
    /// attempt to flush this buffer to the socket.
    pub(crate) fn buffer<U: Buffer>(
        &mut self,
        message: Message<U>,
    ) -> StartSend<GenericMessage, EpSinkError> {
        let message = make_message_body_generic(message)?;
        self.framed_stream.start_send(message)?;
        Ok(AsyncSink::Ready)
        // // Ensure the buffer has capacity.
        // self.wr.reserve(message.required_buffer_size());

        // message.buffer_ref(&mut self.wr)
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), EpSinkError> {
        // // As long as there is buffered data to write, try to write it.
        // while !self.wr.is_empty() {
        //     // Try to write some bytes to the socket
        //     let n = try_ready!(self.socket.poll_write(&self.wr));

        //     // As long as the wr is not empty, a successful write should
        //     // never write 0 bytes.
        //     assert!(n > 0);

        //     // This discards the first `n` bytes of the buffer.
        //     let _ = self.wr.split_to(n);
        // }

        // Ok(Async::Ready(()))
        self.framed_stream.poll_complete()
    }

    // /// Read data from the socket.
    // ///
    // /// This only returns `Ready` when the socket has closed.
    // fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
    //     loop {
    //         // Ensure the read buffer has capacity.
    //         //
    //         // This might result in an internal allocation.
    //         self.rd.reserve(self.buf_size);

    //         // Read data into the buffer.
    //         let n = try_ready!(self.socket.read_buf(&mut self.rd));

    //         if n == 0 {
    //             return Ok(Async::Ready(()));
    //         }
    //     }
    // }

    pub(crate) fn poll_channel(&mut self) -> Poll<(), EndpointError> {
        let _ = self.poll_flush()?;

        // ch.framed_stream
        //     .poll()
        self.receive_future
            .poll()
            .map_err(|e| EndpointError::UnbufferError(e))
    }
}

// impl<T> Stream for EndpointChannel<T>
// where
//     T: Stream<Item = EpStreamItem, Error = EpStreamError>
//         + Sink<SinkItem = EpSinkItem, SinkError = EpSinkError>,
// {
//     type Item = EpStreamItem;
//     type Error = EndpointError;

//     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
//         loop {
//             match try_ready!(self.framed_stream.poll()) {
//                 Some(msg) => {
//                     println!("Received message {:?}", msg);
//                     // do something
//                     return Ok(Async::Ready(Some(msg)));
//                 }
//                 None => {
//                     return Ok(Async::Ready(None));
//                 }
//             }
//         }
//         // loop {
//         //     match self.framed_stream.poll()? {
//         //         Async::NotReady => {
//         //             return Ok(Async::NotReady);
//         //         }
//         //         Async::Ready(message) => {
//         //             println!("Received message {:?}", message);
//         //             if let Some(generic_message) = message {
//         //                 // do something
//         //                 println!("Destructured OK");
//         //             } else {
//         //                 // EOF was reached. The remote has disconnected. There is
//         //                 // nothing more to do.
//         //                 return Ok(Async::Ready(None));
//         //             }
//         //         }
//         //     }
//         // }
//     }
// }

// pub(crate) fn poll_channel<T>(
//     ch: &mut EndpointChannel<T>,
// ) -> Poll<Option<GenericMessage>, EndpointError>
// where
//     T: Stream<Item = EpStreamItem, Error = EpStreamError>
//         + Sink<SinkItem = EpSinkItem, SinkError = EpSinkError>,
// {
//     let _ = ch.poll_flush()?;

//     ch.framed_stream
//         .poll()
//         .map_err(|e| EndpointError::UnbufferError(e))
// }
