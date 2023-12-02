// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Ledger, Mempool};
use crate::{database, vm, LongLivedService, Message, Network};
use anyhow::anyhow;
use async_trait::async_trait;
use node_data::ledger::Transaction;
use node_data::message::{AsyncQueue, Payload, Topics};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

const TOPICS: &[u8] = &[Topics::Tx as u8];

#[derive(Debug)]
enum TxAcceptanceError {
    AlreadyExistsInMempool,
    AlreadyExistsInLedger,
    NullifierExistsInMempool,
    VerificationFailed,
}

impl std::error::Error for TxAcceptanceError {}

impl std::fmt::Display for TxAcceptanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExistsInMempool => {
                write!(f, "this transaction exists in the mempool")
            }
            Self::AlreadyExistsInLedger => {
                write!(f, "this transaction exists in the ledger")
            }
            Self::VerificationFailed => {
                write!(f, "this transaction is invalid")
            }
            Self::NullifierExistsInMempool => {
                write!(f, "this transaction's input(s) exists in the mempool")
            }
        }
    }
}

#[derive(Default)]
pub struct MempoolSrv {
    inbound: AsyncQueue<Message>,
}

pub struct TxFilter {}
impl crate::Filter for TxFilter {
    fn filter(&mut self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Ensure transaction does not exist in the mempool state
        // TODO: Ensure transaction does not exist in blockchain
        // TODO: Check  Nullifier
        Ok(())
    }
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for MempoolSrv
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Add a filter that will discard any transactions invalid to the actual
        // mempool, blockchain state.
        LongLivedService::<N, DB, VM>::add_filter(
            self,
            Topics::Tx.into(),
            Box::new(TxFilter {}),
            &network,
        )
        .await?;

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match &msg.payload {
                    Payload::Transaction(tx) => {
                        if let Err(e) =
                            self.accept_tx::<DB, VM>(&db, &vm, tx).await
                        {
                            tracing::error!("{}", e);
                        } else {
                            network.read().await.broadcast(&msg).await;
                        }
                    }
                    _ => tracing::error!("invalid inbound message payload"),
                }
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "mempool"
    }
}

impl MempoolSrv {
    async fn accept_tx<DB: database::DB, VM: vm::VMExecution>(
        &mut self,
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        tx: &Transaction,
    ) -> anyhow::Result<()> {
        let hash = tx.hash();

        // Perform basic checks on the transaction
        db.read().await.view(|view| {
            // ensure transaction does not exist in the mempool

            if view.get_tx_exists(hash)? {
                return Err(anyhow!(TxAcceptanceError::AlreadyExistsInMempool));
            }

            let nullifiers = tx
                .inner
                .nullifiers()
                .iter()
                .map(|nullifier| nullifier.to_bytes())
                .collect();

            // ensure nullifiers do not exist in the mempool
            if view.get_any_nullifier_exists(nullifiers) {
                return Err(anyhow!(
                    TxAcceptanceError::NullifierExistsInMempool
                ));
            }

            // ensure transaction does not exist in the blockchain
            if view.get_ledger_tx_exists(&hash)? {
                return Err(anyhow!(TxAcceptanceError::AlreadyExistsInLedger));
            }

            Ok(())
        })?;

        // VM Preverify call
        vm.read().await.preverify(tx)?;

        tracing::info!(
            event = "transaction accepted",
            hash = hex::encode(hash)
        );

        // Add transaction to the mempool
        db.read().await.update(|db| db.add_tx(tx))?;

        Ok(())
    }
}
