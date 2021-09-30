// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    endpoints::{poll_and_dispatch, EndpointRx, EndpointStatus, EndpointTx, ToEndpointStatus},
    AsyncReadMessagesExt, MessageStream,
};
use crate::{
    data_types::{ClassOfService, GenericMessage, Message},
    endpoint::*,
    error::to_other_error,
    Result, TranslationTables, TypeDispatcher, VrpnError,
};
use async_std::{net::TcpStream, task::current};
use futures::Sink;
use futures::{channel::mpsc, ready, task, Stream, StreamExt};
use socket2::TcpKeepalive;
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// mock so we can have the member.

#[derive(Debug, Clone)]
struct MessageFramedUdp(());

#[derive(Debug)]
pub struct EndpointIp {
    translation: TranslationTables,
    reliable_tx: Arc<Mutex<EndpointTx<TcpStream>>>,
    reliable_rx: Arc<Mutex<EndpointRx<MessageStream<TcpStream>>>>,
    low_latency_channel: Option<MessageFramedUdp>,
    system_rx: Option<Pin<Box<mpsc::UnboundedReceiver<SystemCommand>>>>,
    system_tx: Option<Pin<Box<mpsc::UnboundedSender<SystemCommand>>>>,
}

impl EndpointIp {
    pub(crate) fn new(reliable_stream: TcpStream) -> EndpointIp {
        let reliable_tx = EndpointTx::new(reliable_stream.clone());
        let reliable_rx = EndpointRx::from_reader(reliable_stream);
        let (system_tx, system_rx) = mpsc::unbounded();
        EndpointIp {
            translation: TranslationTables::new(),
            reliable_tx,
            reliable_rx,
            low_latency_channel: None,
            system_tx: Some(Box::pin(system_tx)),
            system_rx: Some(Box::pin(system_rx)),
        }
    }

    fn poll_system_rx(
        &mut self,
        mut dispatcher: &mut TypeDispatcher,
        cx: &mut Context<'_>,
    ) -> Poll<Result<EndpointStatus>> {
        match self.system_rx.as_mut() {
            Some(rx) => match ready!(rx.as_mut().poll_next(cx)) {
                None => Poll::Ready(Ok(EndpointStatus::Closed)),
                Some(cmd) => {
                    if let Some(cmd) =
                        handle_system_command(&mut dispatcher, self.translation_tables_mut(), cmd)?
                    {
                        match cmd {
                            ExtendedSystemCommand::UdpDescription(desc) => {
                                eprintln!("UdpDescription: {:?}", desc);
                            }
                            ExtendedSystemCommand::LogDescription(desc) => {
                                eprintln!("LogDescription: {:?}", desc);
                            }
                            ExtendedSystemCommand::DisconnectMessage => {
                                eprintln!("DisconnectMessage");
                            }
                        }
                    }
                    Poll::Ready(Ok(EndpointStatus::Open))
                }
            },
            None => Poll::Ready(Ok(EndpointStatus::Closed)),
        }
    }

    pub(crate) fn poll_endpoint(
        &mut self,
        mut dispatcher: &mut TypeDispatcher,
        cx: &mut Context<'_>,
    ) -> Poll<Result<()>> {
        // {

        // let channel_tx_arc = Arc::clone(&self.reliable_tx);
        // let mut channel_tx = channel_tx_arc.lock().map_err(to_other_error)?;
        // let _ = channel_tx.poll_complete()?;
        // }
        let channel_rx_arc = Arc::clone(&self.reliable_rx);
        let mut channel_rx = channel_rx_arc.lock().map_err(to_other_error)?;

        //
        let mut endpoint_status =
            poll_and_dispatch(self, channel_rx.deref_mut(), dispatcher, cx)?.to_endpoint_status();

        // todo UDP here.

        // Now, process the messages we sent ourself.
        loop {
            match self.poll_system_rx(dispatcher, cx) {
                Poll::Ready(Ok(new_status)) => endpoint_status.accumulate_closed(new_status),
                Poll::Ready(Err(e)) => {}
                Poll::Pending => break,
            }
        }

        if endpoint_status == EndpointStatus::Closed {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
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

    fn send_system_change(&self, message: SystemCommand) -> Result<()> {
        println!("send_system_change {:?}", message);
        self.system_tx
            .ok_or(VrpnError::EndpointClosed)?
            .as_mut()
            .unbounded_send(message)
            .map_err(to_other_error)?;
        Ok(())
    }

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()> {
        if class.contains(ClassOfService::RELIABLE) || self.low_latency_channel.is_none() {
            // We either need reliable, or don't have low-latency
            let mut channel = self.reliable_tx.lock().map_err(to_other_error)?;

            match channel
                .start_send(msg)
                .map_err(|e| VrpnError::OtherMessage(e.to_string()))?
            {
                Poll::Ready(_) => Ok(()),
                Poll::Pending => Err(VrpnError::OtherMessage(String::from(
                    "Didn't have room in send buffer",
                ))),
            }
        } else {
            // have and can use low-latency
            unimplemented!()
        }
    }

    fn send_all_descriptions(&mut self, dispatcher: &TypeDispatcher) -> Result<()> {
        let mut messages = dispatcher.pack_all_descriptions()?;
        for msg in messages.into_iter() {
            self.buffer_generic_message(msg, crate::data_types::ClassOfService::RELIABLE)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerInfo;

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn make_endpoint() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let connector = Connect::new(server).expect("should be able to create connection future");

        let _ = connector
            .and_then(|ConnectResults { tcp, udp: _ }| {
                let ep = EndpointIp::new(tcp.unwrap(), None);
                for _i in 0..4 {
                    let _ = ep.reliable_channel.lock()?.poll()?.map(|msg| {
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
            .and_then(|ConnectResults { tcp, udp: _ }| {
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
