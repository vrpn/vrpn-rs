// Copyright 2018, Collabora, Ltd.
// SPDX-License-Identifier: BSL-1.0
// Author: Ryan A. Pavlik <ryan.pavlik@collabora.com>

use crate::async_io::codec::*;
use crate::async_io::cookie::*;
use crate::{ClassOfService, Endpoint, GenericMessage, Result, SystemCommand, TranslationTables};
use futures::channel::mpsc;
use std::fs;
use tokio::fs::File;
use tokio_util::codec::{Decoder, Framed};

pub struct EndpointFile {
    translation: TranslationTables,
    file: Framed<File, FramedMessageCodec>,
    system_rx: mpsc::UnboundedReceiver<SystemCommand>,
    system_tx: mpsc::UnboundedSender<SystemCommand>,
}

impl EndpointFile {
    pub async fn new(file: fs::File) -> Result<EndpointFile> {
        let (system_tx, system_rx) = mpsc::unbounded();
        let mut file = File::from_std(file);
        read_and_check_file_cookie(&mut file).await?;
        Ok(EndpointFile {
            translation: TranslationTables::new(),
            file: FramedMessageCodec.framed(file),
            system_tx,
            system_rx,
        })
    }
}
impl Endpoint for EndpointFile {
    fn translation_tables(&self) -> &TranslationTables {
        &self.translation
    }
    fn translation_tables_mut(&mut self) -> &mut TranslationTables {
        &mut self.translation
    }

    fn send_system_change(&self, _message: SystemCommand) -> Result<()> {
        unimplemented!()
    }

    fn buffer_generic_message(
        &mut self,
        _msg: GenericMessage,
        _class: ClassOfService,
    ) -> Result<()> {
        unimplemented!()
    }
}
