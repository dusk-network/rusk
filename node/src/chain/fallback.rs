// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::{anyhow, Result};
use node_data::ledger::Header;
use std::cmp::Ordering;
use tracing::info;

use crate::{
    database::{self},
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

        info!(
            event = "execute fallback checks",
            height = local.height,
            iter = local.iteration,
            target_iter = remote.iteration,
        );

        self.acc.verify_header_against_local(local, remote).await?;
        self.acc.try_revert(revert_target).await
    }
}
