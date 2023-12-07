// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::{anyhow, Result};
use node_data::ledger::Block;
use tracing::info;

use crate::{
    chain::acceptor,
    database::{self, Ledger},
    vm, Network,
};

use super::acceptor::{Acceptor, RevertTarget};

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
        self.acc.try_revert(RevertTarget::LastFinalizedState).await
    }

    /// Performs a serias of checks to securely allow fallback execution.
    async fn sanity_checks(&self, blk: &Block) -> Result<()> {
        let acc = self.acc;

        let curr_height = acc.get_curr_height().await;
        let curr_iteration = acc.get_curr_iteration().await;

        if curr_height < 1 {
            return Err(anyhow!("cannot fallback over genesis block"));
        }

        if blk.header().iteration > curr_iteration {
            return Err(anyhow!("iteration is higher than current"));
        }

        if blk.header().iteration == curr_iteration {
            // This may happen only if:
            //
            // we have more than one winner blocks per a single iteration, same
            // round.

            // An invalid block was received.
            return Err(anyhow!("iteration is equal to the current"));
        }

        info!(
            event = "starting fallback",
            height = curr_height,
            iter = curr_iteration,
            target_iter = blk.header().iteration,
        );

        let prev_block_height = curr_height - 1;
        let prev_block = acc.db.read().await.view(|v| {
            Ledger::fetch_block_by_height(&v, prev_block_height)?
                .ok_or_else(|| anyhow::anyhow!("could not fetch block"))
        })?;

        info!(
            event = "fallback checking block",
            height = curr_height,
            iter = curr_iteration,
            target_iter = blk.header().iteration,
        );

        // Validate Header/Certificate of the new block upon previous block and
        // provisioners.

        // In an edge case, this may fail on performing fallback between two
        // epochs.
        let provisioners_list = acc.provisioners_list.read().await;
        acceptor::verify_block_header(
            self.acc.db.clone(),
            prev_block.header(),
            &provisioners_list,
            blk.header(),
        )
        .await?;

        Ok(())
    }
}
