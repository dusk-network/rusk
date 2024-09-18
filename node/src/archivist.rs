// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::archive::SQLiteArchive;
use crate::database::Archivist;
use crate::{database, vm, LongLivedService, Network};
use async_trait::async_trait;
use node_data::archive::ArchivalData;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::error;

pub struct ArchivistSrv {
    pub archive_receiver: Receiver<ArchivalData>,
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
            if let Some(msg) = self.archive_receiver.recv().await {
                match msg {
                    ArchivalData::ArchivedEvents(
                        blk_height,
                        blk_hash,
                        events,
                    ) => {
                        if let Err(e) = self
                            .archivist
                            .store_vm_events(blk_height, blk_hash, events)
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
                            .remove_deleted_block(blk_height, hex_blk_hash)
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
                            .mark_block_finalized(blk_height, hex_blk_hash)
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
