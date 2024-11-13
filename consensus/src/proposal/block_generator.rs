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
use crate::operations::{CallParams, Operations, Voter};

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
        // Sign seed
        let seed: [u8; 48] = ru
            .secret_key
            .sign_multisig(ru.pubkey_bls.inner(), &ru.seed().inner()[..])
            .to_bytes();

        let start = Instant::now();

        let candidate = self
            .generate_block(
                ru,
                Seed::from(seed),
                iteration,
                failed_iterations,
                &[],
                ru.att_voters(),
            )
            .await?;

        info!(
            event = "Candidate generated",
            hash = &to_str(&candidate.header().hash),
            gas_limit = candidate.header().gas_limit,
            state_hash = &to_str(&candidate.header().state_hash),
            dur = format!("{:?}ms", start.elapsed().as_millis()),
        );

        let mut candidate_msg = Candidate { candidate };

        candidate_msg.sign(&ru.secret_key, ru.pubkey_bls.inner());

        debug!(event = "Candidate signed", header = ?candidate_msg.candidate.header());

        Ok(candidate_msg.into())
    }

    async fn generate_block(
        &self,
        ru: &RoundUpdate,
        seed: Seed,
        iteration: u8,
        failed_iterations: IterationsInfo,
        faults: &[Fault],
        voters: &[Voter],
    ) -> Result<Block, crate::errors::OperationError> {
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

        let header_size = blk_header.size().map_err(|e| {
            crate::errors::OperationError::InvalidEST(anyhow::anyhow!(
                "Cannot get header size {e}. This should be a bug"
            ))
        })?;

        // We always write the faults len in a u32
        let mut faults_size = u32::SIZE;
        let faults_hashes: Vec<_> = faults
            .iter()
            .map(|f| {
                faults_size += f.size();
                f.hash()
            })
            .collect();

        blk_header.faultroot = merkle_root(&faults_hashes);

        // We know for sure that this operation cannot underflow
        let max_txs_bytes = MAX_BLOCK_SIZE - header_size - faults_size;

        let call_params = CallParams {
            round: ru.round,
            generator_pubkey: ru.pubkey_bls.clone(),
            to_slash,
            voters_pubkey: voters.to_owned(),
            max_txs_bytes,
        };

        let result =
            self.executor.execute_state_transition(call_params).await?;

        blk_header.state_hash = result.verification_output.state_root;
        blk_header.event_bloom = result.verification_output.event_bloom;

        let tx_hashes: Vec<_> =
            result.txs.iter().map(|t| t.inner.hash()).collect();
        let txs: Vec<_> = result.txs.into_iter().map(|t| t.inner).collect();
        blk_header.txroot = merkle_root(&tx_hashes[..]);

        blk_header.timestamp = max(
            ru.timestamp() + *MINIMUM_BLOCK_TIME,
            get_current_timestamp(),
        );

        Block::new(blk_header, txs, faults.to_vec()).map_err(|e| {
            crate::errors::OperationError::InvalidEST(anyhow::anyhow!(
                "Cannot create new block {e}",
            ))
        })
    }
}
