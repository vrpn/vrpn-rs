// Copyright 2018-2019, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

//! A simple, synchronous-IO client for testing purposes.
//!
//! Doesn't use any of the async-io stuff in the vrpn crate,
//! so this is durable even if Tokio totally changes everything.

extern crate bytes;

use crate::{
    buffer_unbuffer::{
        peek_u32, size_requirement::MayContainSizeRequirement, BufferUnbufferError, BytesMutExtras,
        ConstantBufferSize, SizeRequirement, UnbufferFrom,
    },
    data_types::{
        self, id_types::SequenceNumber, message::Message, CookieData, GenericMessage, MessageSize,
        SequencedGenericMessage,
    },
    endpoint::SystemCommand,
    error::VrpnError,
    translation_table::Tables as TranslationTables,
    Endpoint, EndpointGeneric, TypeDispatcher,
};
use bytes::BytesMut;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

/// Write a cookie to a synchronous sink implementing Write.
pub fn write_cookie<T>(stream: &mut T, cookie: CookieData) -> Result<(), VrpnError>
where
    T: Write,
{
    let buf = BytesMut::allocate_and_buffer(cookie)?;
    stream.write_all(&buf.freeze())?;
    Ok(())
}

/// Read a cookie from a synchronous source implementing Read.
pub fn read_cookie<T>(stream: &mut T) -> Result<Vec<u8>, VrpnError>
where
    T: Read,
{
    let mut buf = vec![0u8; CookieData::constant_buffer_size()];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub struct EndpointSyncTcp {
    translation: TranslationTables,
    stream: TcpStream,
    system_rx: mpsc::Receiver<SystemCommand>,
    system_tx: mpsc::Sender<SystemCommand>,
    seq: AtomicUsize,
}

impl EndpointSyncTcp {
    pub fn new(stream: TcpStream) -> EndpointSyncTcp {
        let (system_tx, system_rx) = mpsc::channel();
        EndpointSyncTcp {
            translation: TranslationTables::new(),
            stream,
            system_tx,
            system_rx,
            seq: AtomicUsize::new(0),
        }
    }

    fn read_single_message(&mut self) -> Result<SequencedGenericMessage, VrpnError> {
        self.stream
            .set_read_timeout(Some(Duration::from_millis(1)))?;
        let mut buf = BytesMut::new();

        // Read the message header and padding
        buf.resize(24, 0);
        // Peek the message header and padding
        if let Err(e) = self.stream.peek(buf.as_mut()) {
            match e.kind() {
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                    return Err(VrpnError::BufferUnbuffer(
                        BufferUnbufferError::NeedMoreData(SizeRequirement::Unknown),
                    ));
                }
                // Not a "need more data"
                _ => return Err(VrpnError::Other(Box::new(e))),
            }
        }

        // Peek the size field, to compute the MessageSize.
        let total_len = peek_u32(&buf.clone().freeze()).unwrap();
        let size = MessageSize::try_from_length_field(total_len)?;

        // Read the body of the message
        let mut msg_buf = BytesMut::new();
        msg_buf.resize(size.padded_message_size(), 0);
        self.stream.read_exact(msg_buf.as_mut())?;
        let mut msg_buf = msg_buf.freeze();

        // Unbuffer the message.
        let result = SequencedGenericMessage::try_read_from_buf(&mut msg_buf)?;
        Ok(result)
    }

    pub fn poll_endpoint(&mut self, mut dispatcher: &mut TypeDispatcher) -> Result<(), VrpnError> {
        loop {
            match self.read_single_message() {
                Ok(msg) => {
                    let msg = self.map_remote_message_to_local(msg.into_inner())?;
                    if let Some(msg) = self.passthrough_nonsystem_message(msg)? {
                        dispatcher.call(&msg)?;
                    }
                }
                Err(e) => {
                    if (&e).try_get_size_requirement().is_some() {
                        break;
                    }
                    return Err(e);
                }
            }
        }
        // Now, process the system commands that have been queued.
        loop {
            match self.system_rx.recv_timeout(Duration::from_micros(1)) {
                Ok(cmd) => {
                    if self.handle_system_command(&mut dispatcher, cmd)?.is_some() {
                        // we don't handle any other system commands in this endpoint right now
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // no more commands
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // todo how to handle this? Might never happen.
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Endpoint for EndpointSyncTcp {
    fn translation_tables(&self) -> &TranslationTables {
        &self.translation
    }

    fn translation_tables_mut(&mut self) -> &mut TranslationTables {
        &mut self.translation
    }

    fn send_system_change(&self, message: SystemCommand) -> Result<(), VrpnError> {
        println!("send_system_change {:?}", message);
        self.system_tx
            .send(message)
            .map_err(|e| VrpnError::OtherMessage(e.to_string()))?;
        Ok(())
    }

    fn buffer_generic_message(
        &mut self,
        msg: GenericMessage,
        _class: data_types::ClassOfService,
    ) -> Result<(), VrpnError> {
        // Ignore class of service here
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let sequenced = msg.into_sequenced_message(SequenceNumber(seq as u32));
        let buf = sequenced.try_into_buf()?;

        self.stream.write_all(&buf[..])?;
        Ok(())
    }
}
