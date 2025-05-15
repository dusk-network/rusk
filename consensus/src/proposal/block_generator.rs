// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::max;
use std::sync::Arc;
use std::time::Instant;

use dusk_bytes::Serializable;
use node_data::ledger::{to_str, Block, Fault, IterationsInfo, Seed, Slash};
use node_data::message::payload::Candidate;
use node_data::message::{Message, SignedStepMessage, BLOCK_HEADER_VERSION};
use node_data::{get_current_timestamp, ledger};
use tracing::{debug, info};

use crate::commons::RoundUpdate;
use crate::config::{MAX_BLOCK_SIZE, MAX_NUMBER_OF_FAULTS, MINIMUM_BLOCK_TIME};
use crate::merkle::merkle_root;
use crate::operations::{Operations, StateTransitionData};

pub struct Generator<T: Operations> {
    executor: Arc<T>,
}

impl<T: Operations> Generator<T> {
    pub fn new(executor: Arc<T>) -> Self {
        Self { executor }
    }

    pub async fn generate_candidate_message(
        &self,
        ru: &RoundUpdate,
        iteration: u8,
        failed_iterations: IterationsInfo,
    ) -> Result<Message, crate::errors::OperationError> {
        let candidate = self
            .generate_block(ru, iteration, failed_iterations, &[])
            .await?;

        let mut candidate_msg = Candidate { candidate };

        candidate_msg.sign(&ru.secret_key, ru.pubkey_bls.inner());

        debug!(event = "Candidate signed", header = ?candidate_msg.candidate.header());

        Ok(candidate_msg.into())
    }

    pub async fn generate_block(
        &self,
        ru: &RoundUpdate,
        iteration: u8,
        failed_iterations: IterationsInfo,
        faults: &[Fault],
    ) -> Result<Block, crate::errors::OperationError> {
        let start = Instant::now();

        // Sign seed
        let seed_sig: [u8; 48] = ru
            .secret_key
            .sign_multisig(ru.pubkey_bls.inner(), &ru.seed().inner()[..])
            .to_bytes();
        let seed = Seed::from(seed_sig);

        // Limit number of faults in the block
        let faults = if faults.len() > MAX_NUMBER_OF_FAULTS {
            &faults[..MAX_NUMBER_OF_FAULTS]
        } else {
            faults
        };

        let block_gas_limit = self.executor.get_block_gas_limit().await;
        let to_slash =
            Slash::from_iterations_and_faults(&failed_iterations, faults)?;

        let prev_block_hash = ru.hash();
        let mut blk_header = ledger::Header {
            version: BLOCK_HEADER_VERSION,
            height: ru.round,
            gas_limit: block_gas_limit,
            prev_block_hash,
            seed,
            generator_bls_pubkey: *ru.pubkey_bls.bytes(),
            prev_block_cert: *ru.att(),
            iteration,
            failed_iterations,
            ..Default::default()
        };

        let header_size = blk_header.size();

        // We always write the faults len in a u32
        let mut faults_size = u32::SIZE;
        let fault_digests: Vec<_> = faults
            .iter()
            .map(|f| {
                faults_size += f.size();
                f.digest()
            })
            .collect();

        blk_header.faultroot = merkle_root(&fault_digests);

        // We know for sure that this operation cannot underflow
        let max_txs_bytes = MAX_BLOCK_SIZE - header_size - faults_size;
        let voters = ru.att_voters();

        let transition_data = StateTransitionData {
            round: ru.round,
            generator: ru.pubkey_bls.clone(),
            to_slash,
            voters_pubkey: voters.to_owned(),
            max_txs_bytes,
            prev_state_root: ru.state_root(),
        };

        // Compute a valid state transition for the block
        let (txs, transition_result) = self
            .executor
            .create_state_transition(transition_data)
            .await?;

        blk_header.state_hash = transition_result.state_root;
        blk_header.event_bloom = transition_result.event_bloom;

        let tx_digests: Vec<_> = txs.iter().map(|t| t.inner.digest()).collect();
        let txs: Vec<_> = txs.into_iter().map(|t| t.inner).collect();
        blk_header.txroot = merkle_root(&tx_digests[..]);

        blk_header.timestamp = max(
            ru.timestamp() + *MINIMUM_BLOCK_TIME,
            get_current_timestamp(),
        );

        match Block::new(blk_header, txs, faults.to_vec()) {
            Ok(blk) => {
                info!(
                    event = "Block generated",
                    round = blk.header().height,
                    iter = blk.header().iteration,
                    prev_block = &to_str(&blk.header().prev_block_hash),
                    hash = &to_str(&blk.header().hash),
                    gas_limit = blk.header().gas_limit,
                    state_hash = &to_str(&blk.header().state_hash),
                    dur = format!("{:?}ms", start.elapsed().as_millis()),
                );
                Ok(blk)
            }
            Err(e) => Err(crate::errors::OperationError::BlockCreation(
                format!("{e}",),
            )),
        }
    }
}
