// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use node_data::ledger::{self, Block, Hash, Header};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    database::{self, Ledger},
    vm, Network,
};

use super::acceptor::Acceptor;

pub(crate) struct WithContext<N: Network, DB: database::DB, VM: vm::VMExecution>
{
    acc: Arc<RwLock<Acceptor<N, DB, VM>>>,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> WithContext<N, DB, VM> {
    pub(crate) fn new(acc: Arc<RwLock<Acceptor<N, DB, VM>>>) -> Self {
        Self { acc }
    }

    pub(crate) async fn try_execute_fallback(&self, blk: &Block) -> Result<()> {
        self.sanity_checks(blk).await?;
        self.execute_fallback(blk).await
    }

    /// Performs a serias of checks to securely allow fallback execution.
    async fn sanity_checks(&self, blk: &Block) -> Result<()> {
        let acc = self.acc.read().await;

        let curr_height = acc.get_curr_height().await;
        let curr_iteration = acc.get_curr_iteration().await;

        if curr_height < 1 {
            return Err(anyhow!("cannot fallback over genesis block"));
        }

        if blk.header.iteration > curr_iteration {
            return Err(anyhow!("iteration is higher than current"));
        }

        if blk.header.iteration == curr_iteration {
            // This may happen only if:
            //
            // we have more than one winner blocks per a single iteration, same
            // round.
            //
            // An invalid block was received.
            return Err(anyhow!("iteration is equal to the current"));
        }

        info!(
            "Fallback procedure started curr_iter: {:?} new_iter: {:?}",
            curr_iteration, blk.header.iteration
        );

        // Fetch previous block
        info!("Fetch previous block");

        let prev_block_height = curr_height - 1;
        let mut prev_block = ledger::Block::default();
        acc.db.read().await.view(|v| {
            let hash =
                Ledger::fetch_block_hash_by_height(&v, prev_block_height)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("could not find hash by height")
                    })?;

            prev_block = Ledger::fetch_block(&v, &hash)?
                .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

            Ok(())
        })?;

        info!("Verify block header/certificate data");

        // Validate Header/Certificate of the new block upon previous block
        let empty_public_key = node_data::bls::PublicKey::default();
        acc.verify_block_header(
            prev_block.header(),
            &empty_public_key,
            blk.header(),
        )
        .await
    }

    async fn execute_fallback(&self, blk: &Block) -> Result<()> {
        let acc = self.acc.write().await;
        let curr_height = acc.get_curr_height().await;
        let curr_iteration = acc.get_curr_iteration().await;

        info!("Revert VM to last finalized state");
        let state_root_after_revert = acc.vm.read().await.revert()?;

        // Delete any ephemeral block until we reach the last finalized block,
        // the VM was reverted to.
        info!("Delete all most recent ephemeral blocks");
        acc.db.read().await.update(|t| {
            let mut height = curr_height;
            loop {
                if height == 0 {
                    break;
                }

                let hash = Ledger::fetch_block_hash_by_height(t, height)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("could not find hash by height")
                    })?;

                let chain_blk = Ledger::fetch_block(t, &hash)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

                let iteration = chain_blk.header.iteration;
                if chain_blk.header.state_hash == state_root_after_revert {
                    info!(
                        "state_root found at height: {}, iter: {}",
                        height, iteration
                    );
                    break;
                }

                if iteration == 1 {
                    /// A sanity check to prove we always never delete a
                    /// finalized
                    warn!("deleting a block from first iteration");
                }

                Ledger::delete_block(t, &chain_blk)?;
                height -= 1;
            }

            Ok(())
        })?;

        // Try to inject the block with the lowest iteration
        info!("Inject the new block");

        acc.inject_block(blk).await
    }
}
