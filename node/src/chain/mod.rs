// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod consensus;
mod genesis;

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, bail, Result};

use dusk_consensus::user::committee::CommitteeSet;
use std::sync::Arc;

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

use self::consensus::Task;

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

    /// Most recently accepted block
    most_recent_block: Block,

    /// List of eligible provisioners of actual round
    eligible_provisioners: Provisioners,

    /// Upper layer consensus task
    upper: consensus::Task,
}

impl Drop for ChainSrv {
    fn drop(&mut self) {
        self.upper.abort();
    }
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

        self.init::<N, DB, VM>(&network, &db, &vm).await?;

        loop {
            tokio::select! {
                // Receives results from the upper layer
                recv = &mut self.upper.result.recv() => {
                    if let Ok(res) = recv {
                        match res {
                            Ok(blk) => {
                                if let Err(e) = self.accept_block::<DB, VM>( &db, &vm, &blk).await {
                                    println!("failed to accept block: {} {:#?}", e, blk.header);
                                } else {
                                    //network.read().await.
                                    //    broadcast(&Message::new_with_block(Box::new(blk))).await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("consensus halted due to: {:?}", e);
                            }
                        }
                    }
                },
                // Receives inbound wire messages.
                // Component should either process it or re-route it to the next upper layer
                recv = &mut self.inbound.recv() => {
                    if let Ok(mut msg) = recv {
                        match &msg.payload {
                            Payload::Block(b) => {
                                if let Err(e) = self.accept_block::<DB, VM>(&db, &vm, b).await {
                                    let blk = std::ops::Deref::deref(&b);
                                    tracing::error!("failed to accept block: {}", e);
                                } else {
                                    network.read().await.broadcast(&msg).await;
                                }
                            }

                            // Re-route message to upper layer (in this case it is the Consensus layer)
                            Payload::NewBlock(_) |  Payload::Reduction(_) => {
                                self.upper.main_inbound.try_send(msg);
                            }
                            Payload::Agreement(_) | Payload::AggrAgreement(_) => {
                                self.upper.agreement_inbound.try_send(msg);
                            }
                            _ => tracing::warn!("invalid inbound message"),
                        }
                    }
                },
                // Re-routes messages originated from Consensus (upper) layer to the network layer.
                recv = &mut self.upper.outbound.recv() => {
                    if let Ok(msg) = recv {
                        network.read().await.broadcast(&msg).await;
                    }
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
            upper: Task::new_with_keys(keys_path),
            most_recent_block: Block::default(),
            eligible_provisioners: Provisioners::default(),
        }
    }

    async fn init<N: Network, DB: database::DB, VM: vm::VMExecution>(
        &mut self,
        network: &Arc<RwLock<N>>,
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        (self.most_recent_block, self.eligible_provisioners) =
            genesis::generate_state();

        self.upper.spawn(
            &self.most_recent_block.header,
            &self.eligible_provisioners,
            db,
            vm,
        );

        anyhow::Ok(0)
    }

    async fn accept_block<DB: database::DB, VM: vm::VMExecution>(
        &mut self,
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        blk: &Block,
    ) -> anyhow::Result<()> {
        // Verify Block Header
        self.verify_block_header(
            db,
            &self.most_recent_block.header,
            &blk.header,
        )
        .await?;

        // Reset Consensus
        self.upper.abort();

        // Persist block in consistency with the VM state update
        {
            let vm = vm.read().await;
            db.read().await.update(|t| {
                t.store_block(blk, true)?;

                // Accept block transactions into the VM
                if blk.header.cert.step == 3 {
                    return vm.finalize(blk);
                }

                vm.accept(blk)
            })
        }?;

        self.most_recent_block = blk.clone();

        // Delete from mempool any transaction already included in the block
        db.read().await.update(|update| {
            for tx in blk.txs.iter() {
                database::Mempool::delete_tx(update, tx.hash());
            }
            Ok(())
        })?;

        tracing::info!(
            "block accepted height:{} hash:{} txs_count: {}",
            blk.header.height,
            hex::encode(blk.header.hash),
            blk.txs.len(),
        );

        // Restart Consensus.
        // NB. This will be moved out of accept_block when Synchronizer is
        // implemented.
        self.upper.spawn(
            &self.most_recent_block.header,
            &self.eligible_provisioners,
            db,
            vm,
        );

        Ok(())
    }

    async fn verify_block_header<DB: database::DB>(
        &self,
        db: &Arc<RwLock<DB>>,
        prev_block_header: &ledger::Header,
        blk_header: &ledger::Header,
    ) -> anyhow::Result<()> {
        if blk_header.version > 0 {
            return Err(anyhow!("unsupported block version"));
        }

        if blk_header.height != prev_block_header.height + 1 {
            return Err(anyhow!("invalid block height"));
        }

        if blk_header.prev_block_hash != prev_block_header.hash {
            return Err(anyhow!("invalid previous block hash"));
        }

        if blk_header.timestamp <= prev_block_header.timestamp {
            return Err(anyhow!("invalid block timestamp"));
        }

        // Ensure block is not already in the ledger
        db.read().await.view(|view| {
            if Ledger::get_block_exists(&view, &blk_header.hash)? {
                return Err(anyhow!("block already exists"));
            }

            Ok(())
        })?;

        // TODO: Add check point for MaxBlockTime

        // Verify Certificate
        // NB: Genesis block has no certificate
        if blk_header.height > 0 {
            return self
                .verify_block_cert(
                    blk_header.hash,
                    blk_header.height,
                    &prev_block_header.seed,
                    &blk_header.cert,
                )
                .await;
        }

        Ok(())
    }

    async fn verify_block_cert(
        &self,
        block_hash: [u8; 32],
        height: u64,
        seed: &ledger::Seed,
        cert: &ledger::Certificate,
    ) -> anyhow::Result<()> {
        let (_, public_key) = &self.upper.keys;

        let committee = Arc::new(Mutex::new(CommitteeSet::new(
            public_key.clone(),
            self.eligible_provisioners.clone(),
        )));

        let hdr = node_data::message::Header {
            topic: 0,
            pubkey_bls: public_key.clone(),
            round: height,
            step: cert.step,
            block_hash,
        };

        // Verify first reduction
        if let Err(e) = dusk_consensus::agreement::verifiers::verify_step_votes(
            &cert.first_reduction,
            &committee,
            *seed,
            &hdr,
            0,
        )
        .await
        {
            return Err(anyhow!("ininvalid first reduction votes"));
        }

        // Verify second reduction
        if let Err(e) = dusk_consensus::agreement::verifiers::verify_step_votes(
            &cert.second_reduction,
            &committee,
            *seed,
            &hdr,
            1,
        )
        .await
        {
            return Err(anyhow!("invalid second reduction votes"));
        }

        Ok(())
    }
}
