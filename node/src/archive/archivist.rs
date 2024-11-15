// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use async_trait::async_trait;
use node_data::archive::ArchivalData;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tracing::error;

use crate::archive::Archive;
use crate::{database, vm, LongLivedService, Network};

pub struct ArchivistSrv {
    pub archive_receiver: Receiver<ArchivalData>,
    pub archivist: Archive,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for ArchivistSrv
{
    async fn execute(
        &mut self,
        _: Arc<RwLock<N>>,
        _: Arc<RwLock<DB>>,
        _: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        loop {
            if let Some(msg) = self.archive_receiver.recv().await {
                match msg {
                    ArchivalData::ArchivedEvents(
                        blk_height,
                        blk_hash,
                        events,
                    ) => {
                        if let Err(e) = self
                            .archivist
                            .store_unfinalized_events(
                                blk_height, blk_hash, events,
                            )
                            .await
                        {
                            error!(
                                "Failed to archive block vm events: {:?}",
                                e
                            );
                        }
                    }
                    ArchivalData::DeletedBlock(blk_height, hex_blk_hash) => {
                        if let Err(e) = self
                            .archivist
                            .remove_block_and_events(blk_height, &hex_blk_hash)
                            .await
                        {
                            error!(
                                "Failed to delete block in archive: {:?}",
                                e
                            );
                        }
                    }
                    ArchivalData::FinalizedBlock(blk_height, hex_blk_hash) => {
                        if let Err(e) = self
                            .archivist
                            .finalize_archive_data(blk_height, &hex_blk_hash)
                            .await
                        {
                            error!(
                                "Failed to finalize block in archive: {:?}",
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
        "archivist"
    }
}
