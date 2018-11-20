// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::types::*;
use crate::{
    async_io::{
        codec::{self, FramedMessageCodec},
        endpoint_channel::{poll_and_dispatch, EndpointChannel},
    },
    endpoint::*,
    Error, GenericMessage, MatchingTable, Result, TranslationTables, TypeDispatcher,
};
use futures::sync::mpsc;
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpStream, UdpFramed},
    prelude::*,
};

pub type MessageFramed = codec::MessageFramed<TcpStream>;
pub type MessageFramedUdp = UdpFramed<FramedMessageCodec>;

#[derive(Debug)]
pub struct EndpointIp {
    translation: TranslationTables,
    reliable_channel: Arc<Mutex<EndpointChannel<MessageFramed>>>,
    low_latency_channel: Option<MessageFramedUdp>,
    system_rx: mpsc::UnboundedReceiver<SystemMessage>,
    system_tx: mpsc::UnboundedSender<SystemMessage>,
}
impl EndpointIp {
    pub(crate) fn new(
        reliable_stream: TcpStream,
        low_latency_channel: Option<MessageFramedUdp>,
    ) -> EndpointIp {
        let framed = codec::apply_message_framing(reliable_stream);
        let (system_tx, system_rx) = mpsc::unbounded();
        EndpointIp {
            translation: TranslationTables::new(),
            reliable_channel: EndpointChannel::new(framed),
            low_latency_channel,
            system_tx,
            system_rx,
        }
    }

    pub(crate) fn poll_endpoint(&mut self, dispatcher: &mut TypeDispatcher) -> Poll<(), Error> {
        let channel_arc = Arc::clone(&self.reliable_channel);
        let mut channel = channel_arc
            .lock()
            .map_err(|e| Error::OtherMessage(e.to_string()))?;
        let _ = channel.poll_complete()?;
        let mut closed = poll_and_dispatch(self, channel.deref_mut(), dispatcher)?.is_ready();

        // todo UDP here.

        // Now, process the messages we sent ourself.
        loop {
            let msg_poll = self.system_rx.poll().map_err(|()| {
                Error::OtherMessage(String::from(
                    "error when polling system change message channel",
                ))
            })?;
            match msg_poll {
                Async::Ready(None) => {
                    closed = true;
                    break;
                }
                Async::Ready(Some(msg)) => match msg {
                    SystemMessage::SenderDescription(desc) => {
                        let local_id = dispatcher
                            .register_sender(SenderName(desc.name.clone()))?
                            .get();
                        eprintln!(
                            "Registering sender {:?}: local {:?} = remote {:?}",
                            desc.name, local_id, desc.which
                        );
                        let _ = self.translation.add_remote_entry(
                            desc.name,
                            RemoteId(desc.which),
                            local_id,
                        )?;
                    }
                    SystemMessage::TypeDescription(desc) => {
                        let local_id = dispatcher.register_type(TypeName(desc.name.clone()))?.get();
                        eprintln!(
                            "Registering type {:?}: local {:?} = remote {:?}",
                            desc.name, local_id, desc.which
                        );
                        let _ = self.translation.add_remote_entry(
                            desc.name,
                            RemoteId(desc.which),
                            local_id,
                        )?;
                    }
                    SystemMessage::UdpDescription(desc) => {
                        eprintln!("UdpDescription: {:?}", desc);
                    }
                    SystemMessage::LogDescription(desc) => {
                        eprintln!("LogDescription: {:?}", desc);
                    }
                    SystemMessage::DisconnectMessage => {
                        eprintln!("DesconnectMessage");
                    }
                },
                Async::NotReady => break,
            }
        }

        if closed {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl Endpoint for EndpointIp {
    fn translation_tables(&self) -> &TranslationTables {
        &self.translation
    }

    fn translation_tables_mut(&mut self) -> &mut TranslationTables {
        &mut self.translation
    }

    fn send_system_change(&self, message: SystemMessage) -> Result<()> {
        println!("send_system_change {:?}", message);
        self.system_tx
            .unbounded_send(message)
            .map_err(|e| Error::OtherMessage(e.to_string()))?;
        Ok(())
    }

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()> {
        if class.contains(ServiceFlags::RELIABLE) || self.low_latency_channel.is_none() {
            // We either need reliable, or don't have low-latency
            let mut channel = self
                .reliable_channel
                .lock()
                .map_err(|e| Error::OtherMessage(e.to_string()))?;
            match channel
                .start_send(msg)
                .map_err(|e| Error::OtherMessage(e.to_string()))?
            {
                AsyncSink::Ready => Ok(()),
                AsyncSink::NotReady(_) => Err(Error::OtherMessage(String::from(
                    "Didn't have room in send buffer",
                ))),
            }
        } else {
            // have and can use low-latency
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        async_io::{
            apply_message_framing,
            connect::{Connect, ConnectResults},
        },
        ServerInfo,
    };

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn make_endpoint() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let connector = Connect::new(server).expect("should be able to create connection future");

        let _ = connector
            .and_then(|ConnectResults { tcp, udp }| {
                let ep = EndpointIp::new(tcp.unwrap(), None);
                for _i in 0..4 {
                    let _ = ep
                        .reliable_channel
                        .lock()
                        .unwrap()
                        .poll()
                        .unwrap()
                        .map(|msg| {
                            eprintln!("Received message {:?}", msg);
                            msg
                        });
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn run_endpoint() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let connector = Connect::new(server).expect("should be able to create connection future");

        let _ = connector
            .and_then(|ConnectResults { tcp, udp }| {
                let mut ep = EndpointIp::new(tcp.unwrap(), None);
                let mut disp = TypeDispatcher::new();
                for _i in 0..4 {
                    let _ = ep.poll_endpoint(&mut disp).unwrap();
                }
                Ok(())
            })
            .wait()
            .unwrap();
    }
}
