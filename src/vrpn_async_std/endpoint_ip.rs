// Copyright 2018-2021, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use super::{
    endpoints::{merge_status, poll_and_dispatch, EndpointRx, EndpointStatus, ToEndpointStatus},
    AsyncReadMessagesExt, MessageStream, UnboundedMessageSender,
};
use crate::{
    data_types::{ClassOfService, GenericMessage, Message},
    endpoint::*,
    error::to_other_error,
    Result, TranslationTables, TypeDispatcher, VrpnError,
};
use async_std::{
    net::{TcpStream, UdpSocket},
    task::current,
};
use futures::{channel::mpsc, ready, task, Future, Stream, StreamExt};
use futures::{future::BoxFuture, Sink};
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

#[derive(Debug)]
struct MessageFramedUdp(UdpSocket);

#[derive(Debug)]
pub struct EndpointIp {
    translation: TranslationTables,
    reliable_tx: Pin<Box<UnboundedMessageSender>>,
    reliable_rx: Arc<Mutex<EndpointRx<MessageStream<TcpStream>>>>,
    low_latency_channel: Option<MessageFramedUdp>,
    system_rx: Option<Pin<Box<mpsc::UnboundedReceiver<SystemCommand>>>>,
    system_tx: Option<Pin<Box<mpsc::UnboundedSender<SystemCommand>>>>,
}

impl EndpointIp {
    pub(crate) fn new(reliable_stream: TcpStream, udp: Option<UdpSocket>) -> EndpointIp {
        let reliable_tx = UnboundedMessageSender::new(reliable_stream.clone());
        let reliable_rx = EndpointRx::from_reader(reliable_stream);
        let (system_tx, system_rx) = mpsc::unbounded();
        EndpointIp {
            translation: TranslationTables::new(),
            reliable_tx,
            reliable_rx,
            low_latency_channel: udp.map(|x| MessageFramedUdp(x)),
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
        dispatcher: &mut TypeDispatcher,
        cx: &mut Context<'_>,
    ) -> Poll<Result<()>> {
        let channel_rx_arc = Arc::clone(&self.reliable_rx);
        let mut channel_rx = channel_rx_arc.lock().map_err(to_other_error)?;

        //
        let mut endpoint_status =
            poll_and_dispatch(self, channel_rx.deref_mut(), dispatcher, cx).to_endpoint_status();

        match self.reliable_tx.as_mut().poll(cx) {
            Poll::Ready(Ok(())) => {
                println!("Remote end of reliable connection has shut down.");
                endpoint_status = merge_status(endpoint_status, EndpointStatus::Closed);
            }
            Poll::Ready(Err(e)) => endpoint_status = EndpointStatus::ClosedError(e),
            Poll::Pending => {}
        }
        // todo UDP here.

        // Now, process the messages we sent ourself.
        loop {
            match self.poll_system_rx(dispatcher, cx) {
                Poll::Ready(Ok(new_status)) => {
                    endpoint_status = merge_status(endpoint_status, new_status)
                }
                Poll::Ready(Err(_e)) => {}
                Poll::Pending => break,
            }
        }
        if endpoint_status.is_closed() {
            self.reliable_tx.close();
        }

        endpoint_status.into()
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
        if let Some(tx) = self.system_tx.clone().as_deref_mut() {
            tx.unbounded_send(message).map_err(to_other_error)?;
        }
        Ok(())
    }

    fn buffer_generic_message(&mut self, msg: GenericMessage, class: ClassOfService) -> Result<()> {
        if class.contains(ClassOfService::RELIABLE) || self.low_latency_channel.is_none() {
            // We either need reliable, or don't have low-latency
            self.reliable_tx.as_mut().unbounded_send(msg)
        } else {
            // have and can use low-latency
            unimplemented!()
        }
    }

    fn send_all_descriptions(&mut self, dispatcher: &TypeDispatcher) -> Result<()> {
        let messages = dispatcher.pack_all_descriptions()?;
        for msg in messages.into_iter() {
            self.buffer_generic_message(msg, crate::data_types::ClassOfService::RELIABLE)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vrpn_async_std::cookie;
    use crate::ServerInfo;
    use async_std::net::{TcpStream, ToSocketAddrs};
    use futures::executor::block_on;
    use futures::prelude::*;
    use std::net::IpAddr;

    async fn connect_and_handshake(server_info: ServerInfo) -> crate::Result<TcpStream> {
        let mut stream = TcpStream::connect(server_info.socket_addr).await?;
        stream.set_nodelay(true)?;

        // We first write our cookie, then read and check the server's cookie, before the loop.
        cookie::send_nonfile_cookie(&mut stream).await?;
        cookie::read_and_check_nonfile_cookie(&mut stream).await?;
        Ok(stream)
    }
    #[ignore] // because it requires an external server to be running.
    #[test]
    fn make_endpoint() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let result: Result<EndpointIp> = block_on(async {
            let tcp = connect_and_handshake(server).await?;
            Ok(EndpointIp::new(tcp, None))
        });
        result.unwrap();
    }

    #[ignore] // because it requires an external server to be running.
    #[test]
    fn run_endpoint() {
        let server = "tcp://127.0.0.1:3883".parse::<ServerInfo>().unwrap();
        let result: Result<()> = block_on(async {
            let tcp = connect_and_handshake(server).await.unwrap();

            let ep = EndpointIp::new(tcp, None);
            let rx = Arc::clone(&ep.reliable_rx);
            for _i in 0..4 {
                let msg = rx
                    .lock()?
                    .next()
                    .await
                    .ok_or(VrpnError::GenericErrorReturn)?;
                eprintln!("Received message {:?}", msg);
            }
            Ok(())
        });
        result.unwrap();
    }
}
