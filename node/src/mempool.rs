// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod conf;

use crate::database::{Ledger, Mempool};
use crate::mempool::conf::Params;
use crate::{database, vm, LongLivedService, Message, Network};
use async_trait::async_trait;
use node_data::events::{Event, TransactionEvent};
use node_data::ledger::Transaction;
use node_data::message::{AsyncQueue, Payload, Topics};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const TOPICS: &[u8] = &[Topics::Tx as u8];

#[derive(Debug, Error)]
enum TxAcceptanceError {
    #[error("this transaction exists in the mempool")]
    AlreadyExistsInMempool,
    #[error("this transaction exists in the ledger")]
    AlreadyExistsInLedger,
    #[error("this transaction's input(s) exists in the mempool")]
    NullifierExistsInMempool,
    #[error("this transaction is invalid {0}")]
    VerificationFailed(String),
    #[error("A generic error occurred {0}")]
    Generic(anyhow::Error),
}

impl From<anyhow::Error> for TxAcceptanceError {
    fn from(err: anyhow::Error) -> Self {
        Self::Generic(err)
    }
}

pub struct MempoolSrv {
    inbound: AsyncQueue<Message>,
    event_sender: Sender<Event>,
}

impl MempoolSrv {
    pub fn new(conf: Params, event_sender: Sender<Event>) -> Self {
        info!("MempoolSrv::new with conf {}", conf);
        Self {
            inbound: AsyncQueue::bounded(
                conf.max_queue_size,
                "mempool_inbound",
            ),
            event_sender,
        }
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

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match &msg.payload {
                    Payload::Transaction(tx) => {
                        let accept = self.accept_tx::<DB, VM>(&db, &vm, tx);
                        if let Err(e) = accept.await {
                            error!("{}", e);
                            continue;
                        }

                        let network = network.read().await;
                        if let Err(e) = network.broadcast(&msg).await {
                            warn!("Unable to broadcast accepted tx: {e}")
                        };
                    }
                    _ => error!("invalid inbound message payload"),
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
    ) -> Result<(), TxAcceptanceError> {
        // VM Preverify call
        if let Err(e) = vm.read().await.preverify(tx) {
            Err(TxAcceptanceError::VerificationFailed(format!("{e:?}")))?;
        }

        let tx_id = tx.id();

        // Perform basic checks on the transaction
        db.read().await.view(|view| {
            // ensure transaction does not exist in the mempool

            if view.get_tx_exists(tx_id)? {
                return Err(TxAcceptanceError::AlreadyExistsInMempool);
            }

            let nullifiers: Vec<_> = tx
                .inner
                .nullifiers()
                .iter()
                .map(|nullifier| nullifier.to_bytes())
                .collect();

            // ensure nullifiers do not exist in the mempool
            for m_tx_id in view.get_txs_by_nullifiers(&nullifiers) {
                if let Some(m_tx) = view.get_tx(m_tx_id)? {
                    if m_tx.inner.gas_price() < tx.inner.gas_price() {
                        if view.delete_tx(m_tx_id)? {
                            let node_event = TransactionEvent::Removed(m_tx_id).into();
                            if let Err(e) = self.event_sender.try_send(node_event) {
                                warn!("cannot notify mempool accepted transaction {e}")
                            };
                        }
                    } else {
                        return Err(
                            TxAcceptanceError::NullifierExistsInMempool,
                        );
                    }
                }
            }

            // ensure transaction does not exist in the blockchain
            if view.get_ledger_tx_exists(&tx_id)? {
                return Err(TxAcceptanceError::AlreadyExistsInLedger);
            }

            Ok(())
        })?;

        tracing::info!(
            event = "transaction accepted",
            hash = hex::encode(tx_id)
        );

        // Add transaction to the mempool
        db.read().await.update(|db| db.add_tx(tx))?;

        let node_event = TransactionEvent::Included(tx).into();

        if let Err(e) = self.event_sender.try_send(node_event) {
            warn!("cannot notify mempool accepted transaction {e}")
        };

        Ok(())
    }
}
