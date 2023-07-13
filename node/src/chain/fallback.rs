// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use node_data::{
    bls::PublicKey,
    ledger::{self, Block, Hash, Header},
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    chain::acceptor,
    database::{self, Ledger, Mempool},
    vm, Network,
};

use super::acceptor::Acceptor;

/// Wraps up any handlers or data needed by fallback to complete.
pub(crate) struct WithContext<
    'a,
    N: Network,
    DB: database::DB,
    VM: vm::VMExecution,
> {
    acc: &'a Acceptor<N, DB, VM>,
}

impl<'a, N: Network, DB: database::DB, VM: vm::VMExecution>
    WithContext<'a, N, DB, VM>
{
    pub(crate) fn new(acc: &'a Acceptor<N, DB, VM>) -> Self {
        Self { acc }
    }

    pub(crate) async fn try_execute_fallback(&self, blk: &Block) -> Result<()> {
        self.sanity_checks(blk).await?;
        self.execute_fallback(blk).await
    }

    /// Performs a serias of checks to securely allow fallback execution.
    async fn sanity_checks(&self, blk: &Block) -> Result<()> {
        let acc = self.acc;

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
            prev_block = Ledger::fetch_block_by_height(&v, prev_block_height)?
                .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

            Ok(())
        })?;

        info!("Verify block header/certificate data");

        // Validate Header/Certificate of the new block upon previous block and
        // provisioners.

        // In an edge case, this may fail on performing fallback between two
        // epochs.
        let provisioners_list = acc.provisioners_list.read().await;
        acceptor::verify_block_header(
            self.acc.db.clone(),
            prev_block.header(),
            provisioners_list.clone(),
            &PublicKey::default(),
            blk.header(),
        )
        .await
    }

    async fn execute_fallback(&self, blk: &Block) -> Result<()> {
        let acc = self.acc;
        let curr_height = acc.get_curr_height().await;
        let curr_iteration = acc.get_curr_iteration().await;

        info!("Revert VM to last finalized state");
        let state_hash_after_revert = acc.vm.read().await.revert()?;

        info!(
            "Revert completed, finalized_state_hash:{}",
            hex::ToHex::encode_hex::<String>(&state_hash_after_revert)
        );

        // Delete any ephemeral block until we reach the last finalized block,
        // the VM was reverted to.
        info!("Delete all most recent ephemeral blocks");

        // Thew blockchain tip (most recent block) after reverting to last
        // finalized state.
        let mut new_mrb = Block::default();

        acc.db.read().await.update(|t| {
            let mut height = curr_height;
            loop {
                if height == 0 {
                    break;
                }

                let chain_blk = Ledger::fetch_block_by_height(t, height)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

                let iteration = chain_blk.header.iteration;
                if chain_blk.header.state_hash == state_hash_after_revert {
                    info!(
                        "state_hash found at height: {}, iter: {}",
                        height, iteration
                    );

                    new_mrb = chain_blk;
                    break;
                }

                if iteration == 1 {
                    /// A sanity check to ensure we never delete a finalized
                    /// block
                    warn!("deleting a block from first iteration");
                }

                Ledger::delete_block(t, &chain_blk)?;

                // Attempt to resubmit transactions back to mempool.
                // An error here is not considered critical.
                for tx in blk.txs().iter() {
                    Mempool::add_tx(t, tx).map_err(|err| {
                        tracing::error!("failed to resubmit transactions")
                    });
                }

                height -= 1;
            }

            Ok(())
        })?;

        // Update blockchain tip to be the one we reverted to.
        info!("Set new most_recent_block");

        acc.update_most_recent_block(&new_mrb).await
    }
}
