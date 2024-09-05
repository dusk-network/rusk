// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use crate::operations::{CallParams, Operations, Voter};
use node_data::ledger::{
    to_str, Attestation, Block, Fault, IterationsInfo, Seed, Slash,
};
use std::cmp::max;

use crate::merkle::merkle_root;

use crate::config::MINIMUM_BLOCK_TIME;
use dusk_bytes::Serializable;
use node_data::message::payload::Candidate;
use node_data::message::{ConsensusHeader, Message, SignInfo, StepMessage};
use node_data::{get_current_timestamp, ledger};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

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
    ) -> Result<Message, crate::operations::Error> {
        // Sign seed
        let seed = ru
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
            event = "gen_candidate",
            hash = &to_str(&candidate.header().hash),
            gas_limit = candidate.header().gas_limit,
            state_hash = &to_str(&candidate.header().state_hash),
            dur = format!("{:?}ms", start.elapsed().as_millis()),
        );

        debug!("block: {:?}", &candidate);

        let header = ConsensusHeader {
            prev_block_hash: ru.hash(),
            round: ru.round,
            iteration,
        };
        let sign_info = SignInfo::default();
        let mut candidate = Candidate {
            header,
            candidate,
            sign_info,
        };

        candidate.sign(&ru.secret_key, ru.pubkey_bls.inner());

        Ok(candidate.into())
    }

    async fn generate_block(
        &self,
        ru: &RoundUpdate,
        seed: Seed,
        iteration: u8,
        failed_iterations: IterationsInfo,
        faults: &[Fault],
        voters: &[Voter],
    ) -> Result<Block, crate::operations::Error> {
        let to_slash =
            Slash::from_iterations_and_faults(&failed_iterations, faults)?;

        let call_params = CallParams {
            round: ru.round,
            generator_pubkey: ru.pubkey_bls.clone(),
            to_slash,
            voters_pubkey: Some(voters.to_owned()),
        };

        let result =
            self.executor.execute_state_transition(call_params).await?;

        let block_gas_limit = self.executor.get_block_gas_limit().await;

        let tx_hashes: Vec<_> =
            result.txs.iter().map(|t| t.inner.hash()).collect();
        let txs: Vec<_> = result.txs.into_iter().map(|t| t.inner).collect();
        let txroot = merkle_root(&tx_hashes[..]);

        let faults = Vec::<Fault>::new();
        let faults_hashes: Vec<_> = faults.iter().map(|f| f.hash()).collect();
        let faultroot = merkle_root(&faults_hashes);
        let timestamp =
            max(ru.timestamp() + MINIMUM_BLOCK_TIME, get_current_timestamp());

        let prev_block_hash = ru.hash();
        let blk_header = ledger::Header {
            version: 0,
            height: ru.round,
            timestamp,
            gas_limit: block_gas_limit,
            prev_block_hash,
            seed,
            generator_bls_pubkey: *ru.pubkey_bls.bytes(),
            state_hash: result.verification_output.state_root,
            event_hash: result.verification_output.event_hash,
            hash: [0; 32],
            att: Attestation::default(),
            prev_block_cert: *ru.att(),
            txroot,
            faultroot,
            iteration,
            failed_iterations,
        };

        Ok(Block::new(blk_header, txs, faults).expect("block should be valid"))
    }
}
