// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::committee::CommitteeSet;
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, Block, Hash, Header};
use node_data::message::AsyncQueue;
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;

/// Implements block acceptance procedure. This includes block header,
/// certificate and transactions full verifications.
pub(crate) struct BlockAcceptor<
    DB: database::DB,
    VM: vm::VMExecution,
    N: Network,
> {
    /// Most recently accepted block a.k.a blockchain tip
    mrb: RefCell<Block>,

    /// List of eligible provisioners of actual round
    eligible_provisioners: RefCell<Provisioners>,

    db: Arc<RwLock<DB>>,
    vm: Arc<RwLock<VM>>,
    network: Arc<RwLock<N>>,

    /// Upper layer consensus task
    upper: RefCell<super::consensus::Task>,
}

impl<DB: database::DB, VM: vm::VMExecution, N: Network>
    BlockAcceptor<DB, VM, N>
{
    pub(crate) async fn try_accept_block(
        &mut self,
        blk: &Block,
    ) -> anyhow::Result<()> {
        let (_, public_key) = &self.upper.borrow().keys;

        // Verify Block Header
        self.verify_block_header(
            &public_key,
            &self.eligible_provisioners.borrow(),
            &self.mrb.borrow().header,
            &blk.header,
        )
        .await?;

        // Reset Consensus
        self.upper.borrow_mut().abort().await;

        // Persist block in consistency with the VM state update
        {
            let vm = self.vm.read().await;
            self.db.read().await.update(|t| {
                t.store_block(blk, true)?;

                // Accept block transactions into the VM
                if blk.header.cert.step == 3 {
                    return vm.finalize(blk);
                }

                vm.accept(blk)
            })
        }?;

        *self.mrb.borrow_mut() = blk.clone();

        // Delete from mempool any transaction already included in the block
        self.db.read().await.update(|update| {
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
        self.upper.borrow_mut().spawn(
            &self.mrb.borrow().header,
            &self.eligible_provisioners.borrow(),
            &self.db,
            &self.vm,
            &self.network,
        );

        Ok(())
    }

    pub(crate) async fn verify_block_header(
        &self,
        public_key: &node_data::bls::PublicKey,
        eligible_provisioners: &Provisioners,
        prev_block_header: &ledger::Header,
        blk_header: &ledger::Header,
    ) -> anyhow::Result<()> {
        if blk_header.version > 0 {
            return Err(anyhow!("unsupported block version"));
        }

        if blk_header.height != prev_block_header.height + 1 {
            return Err(anyhow!(
                "invalid block height block_height: {:?}, curr_height: {:?}",
                blk_header.height,
                prev_block_header.height,
            ));
        }

        if blk_header.prev_block_hash != prev_block_header.hash {
            return Err(anyhow!("invalid previous block hash"));
        }

        if blk_header.timestamp <= prev_block_header.timestamp {
            return Err(anyhow!("invalid block timestamp"));
        }

        // Ensure block is not already in the ledger
        self.db.read().await.view(|view| {
            if Ledger::get_block_exists(&view, &blk_header.hash)? {
                return Err(anyhow!("block already exists"));
            }

            Ok(())
        })?;

        // Verify Certificate
        Self::verify_block_cert(
            public_key,
            eligible_provisioners,
            blk_header.hash,
            blk_header.height,
            &prev_block_header.seed,
            &blk_header.cert,
        )
        .await
    }

    async fn verify_block_cert(
        public_key: &node_data::bls::PublicKey,
        eligible_provisioners: &Provisioners,
        block_hash: [u8; 32],
        height: u64,
        seed: &ledger::Seed,
        cert: &ledger::Certificate,
    ) -> anyhow::Result<()> {
        let committee = Arc::new(Mutex::new(CommitteeSet::new(
            public_key.clone(),
            eligible_provisioners.clone(),
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
            return Err(anyhow!("invalid first reduction votes"));
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
