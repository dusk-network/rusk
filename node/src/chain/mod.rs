// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod acceptor;
mod consensus;
mod fsm;
mod genesis;

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, bail, Result};

use dusk_consensus::user::committee::CommitteeSet;
use std::rc::Rc;
use std::sync::Arc;
use tracing::{debug, error, warn};

use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, Block, Hash, Header};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;

use self::acceptor::Acceptor;
use self::consensus::Task;
use self::fsm::SimpleFSM;

const TOPICS: &[u8] = &[
    Topics::Block as u8,
    Topics::NewBlock as u8,
    Topics::Reduction as u8,
    Topics::AggrAgreement as u8,
    Topics::Agreement as u8,
];

pub struct ChainSrv {
    /// Inbound wire messages queue
    inbound: AsyncQueue<Message>,
    keys_path: String,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for ChainSrv
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Restore/Load most recent block
        let (mrb, provisioners_list) =
            Self::load_most_recent_block(db.clone()).await?;

        // Initialize Acceptor and trigger consensus task
        let acc = Arc::new(RwLock::new(
            Acceptor::new_with_run(
                &self.keys_path,
                &mrb,
                &provisioners_list,
                db.clone(),
                network.clone(),
                vm.clone(),
            )
            .await,
        ));

        // Start-up FSM instance
        let mut fsm = SimpleFSM::new(acc.clone(), network.clone());

        let outbound_chan = acc.read().await.get_outbound_chan().await;
        let result_chan = acc.read().await.get_result_chan().await;

        // Message loop for Chain context
        loop {
            tokio::select! {
                // Receives results from the upper layer
                recv = &mut result_chan.recv() => {
                    match recv? {
                        Ok(blk) => {
                            fsm.on_event(&blk, &Message::new_block(Box::new(blk.clone())))
                                .await
                                .map_err(|e| error!("failed to process block: {}", e));
                        }
                        Err(e) => {
                             warn!("consensus err: {:?}", e);
                        }
                    }
                },
                // Receives inbound wire messages.
                // Component should either process it or re-route it to the next upper layer
                recv = &mut self.inbound.recv() => {
                    let msg = recv?;
                    match &msg.payload {
                        Payload::Block(blk) => {
                            debug!("received block {:?}", blk);

                            fsm.on_event(blk, &msg)
                                .await
                                .map_err(|e| error!("failed to process block: {}", e));
                        }

                        // Re-route message to the acceptor
                        Payload::NewBlock(_)
                        | Payload::Reduction(_)
                        | Payload::Agreement(_)
                        | Payload::AggrAgreement(_) => {
                            acc.read().await.reroute_msg(msg).await;
                        }
                        _ => warn!("invalid inbound message"),
                    }
                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = &mut outbound_chan.recv() => {
                    let msg = recv?;
                    network.read().await.broadcast(&msg).await;
                },
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "chain"
    }
}

impl ChainSrv {
    pub fn new(keys_path: String) -> Self {
        Self {
            inbound: Default::default(),
            keys_path,
        }
    }

    /// Load most recent block from persisted ledger.
    ///
    /// Panics
    ///
    /// If register entry is read but block is not found.
    async fn load_most_recent_block<DB: database::DB>(
        db: Arc<RwLock<DB>>,
    ) -> Result<(Block, Provisioners)> {
        let mut mrb = Block::default();

        db.read().await.update(|t| {
            mrb = {
                if let Some(r) = t.get_register()? {
                    t.fetch_block(&r.mrb_hash)?.unwrap()
                } else {
                    // No register is considered malformed or empty database
                    let genesis_blk = genesis::generate_state();

                    /// Persist gensis block
                    t.store_block(&genesis_blk, true);
                    genesis_blk
                }
            };

            Ok(())
        });

        tracing::info!("loaded block height: {}", mrb.header.height);

        // TODO: Until Rusk API is integrated, the list of eligible provisioners
        // is always hard-coded.
        Ok((mrb, genesis::get_mocked_provisioners()))
    }
}
