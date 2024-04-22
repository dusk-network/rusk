// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::{anyhow, Result};
use dusk_consensus::user::provisioners::ContextProvisioners;
use node_data::ledger::{to_str, Header};
use std::cmp::Ordering;
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

    /// Makes an attempt to revert to the specified Target, if remote header is
    /// fully valid
    pub(crate) async fn try_revert(
        &self,
        local: &Header,
        remote: &Header,
        revert_target: RevertTarget,
    ) -> Result<()> {
        self.verify_header(local, remote).await?;
        self.acc.try_revert(revert_target).await
    }

    /// Verifies if a block with header `local` can be replaced with a block
    /// with header `remote`
    async fn verify_header(
        &self,
        local: &Header,
        remote: &Header,
    ) -> Result<()> {
        match (local.height, remote.iteration.cmp(&local.iteration)) {
            (0, _) => Err(anyhow!("cannot fallback over genesis block")),
            (_, Ordering::Greater) => Err(anyhow!(
                "iteration {:?} is higher than the current {:?}",
                remote.iteration,
                local.iteration
            )),
            (_, Ordering::Equal) => Err(anyhow!(
                "iteration is equal to the current {:?}",
                local.iteration
            )), // TODO: This may be a slashing condition
            _ => Ok(()),
        }?;

        let prev_header = self.acc.db.read().await.view(|t| {
            let prev_hash = &local.prev_block_hash;
            t.fetch_block_header(prev_hash)?
                .map(|(header, _)| header)
                .ok_or(anyhow::anyhow!(
                    "Unable to find block with hash {}",
                    to_str(prev_hash)
                ))
        })?;

        info!(
            event = "execute fallback checks",
            height = local.height,
            iter = local.iteration,
            target_iter = remote.iteration,
        );

        let provisioners_list = self
            .acc
            .vm
            .read()
            .await
            .get_provisioners(prev_header.state_hash)?;

        let mut provisioners_list = ContextProvisioners::new(provisioners_list);

        let changed_provisioners = self
            .acc
            .vm
            .read()
            .await
            .get_changed_provisioners(prev_header.state_hash)?;
        provisioners_list.apply_changes(changed_provisioners);

        // Ensure header of the new block is valid according to prev_block
        // header
        let _ = acceptor::verify_block_header(
            self.acc.db.clone(),
            &prev_header,
            &provisioners_list,
            remote,
        )
        .await?;

        Ok(())
    }
}
