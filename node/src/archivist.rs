// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::panic;
use std::sync::{mpsc, Arc};

use async_trait::async_trait;
use node_data::archive::ArchivalData;
use tokio::sync::Mutex;
use tracing::error;

use crate::database::archive::SQLiteArchive;
use crate::database::Archivist;
use crate::{database, vm, LongLivedService, Network};

pub struct ArchivistSrv {
    pub archive_receiver: Mutex<mpsc::Receiver<ArchivalData>>,
    pub archivist: SQLiteArchive,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for ArchivistSrv
{
    async fn execute(
        &mut self,
        _: Arc<tokio::sync::RwLock<N>>,
        _: Arc<tokio::sync::RwLock<DB>>,
        _: Arc<tokio::sync::RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        loop {
            if let Ok(msg) = self.archive_receiver.lock().await.recv() {
                match msg {
                    ArchivalData::ArchivedEvents(
                        block_height,
                        block_hash,
                        events,
                    ) => {
                        if let Err(e) = self
                            .archivist
                            .store_vm_events(block_height, block_hash, events)
                            .await
                        {
                            error!(
                                "Failed to archive block vm events: {:?}",
                                e
                            );
                        }
                    }
                }
            } else {
                error!(
                    "Sending side of the archive data channel has been closed"
                );

                break;
            }
        }

        Ok(0)
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "archive"
    }
}
