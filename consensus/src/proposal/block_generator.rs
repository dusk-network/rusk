// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{IterCounter, RoundUpdate};
use crate::contract_state::CallParams;
use node_data::ledger::{to_str, Block, Certificate, IterationsInfo, Seed};

use crate::config;
use crate::contract_state::Operations;
use crate::merkle::merkle_root;

use dusk_bytes::Serializable;
use node_data::ledger;
use node_data::message::payload::Candidate;
use node_data::message::{Header, Message, Topics};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, info};

pub struct Generator<T: Operations> {
    executor: Arc<Mutex<T>>,
}

impl<T: Operations> Generator<T> {
    pub fn new(executor: Arc<Mutex<T>>) -> Self {
        Self { executor }
    }

    pub async fn generate_candidate_message(
        &self,
        ru: &RoundUpdate,
        step: u8,
        failed_iterations: Vec<Option<Certificate>>,
    ) -> Result<Message, crate::contract_state::Error> {
        let iteration = u8::from_step(step);
        // Sign seed
        let seed = ru
            .secret_key
            .sign(ru.pubkey_bls.inner(), &ru.seed().inner()[..])
            .to_bytes();

        let start = Instant::now();

        let candidate = self
            .generate_block(ru, Seed::from(seed), iteration, failed_iterations)
            .await?;

        info!(
            event = "gen_candidate",
            hash = &to_str(&candidate.header().hash),
            state_hash = &to_str(&candidate.header().state_hash),
            dur = format!("{:?}ms", start.elapsed().as_millis()),
        );

        debug!("block: {:?}", &candidate);

        let msg_header = Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            block_hash: candidate.header().hash,
            step,
            topic: Topics::Candidate as u8,
        };

        let signature = msg_header.sign(&ru.secret_key, ru.pubkey_bls.inner());

        Ok(Message::new_newblock(
            msg_header,
            Candidate {
                candidate,
                signature,
            },
        ))
    }

    async fn generate_block(
        &self,
        ru: &RoundUpdate,
        seed: Seed,
        iteration: u8,
        failed_iterations: Vec<Option<Certificate>>,
    ) -> Result<Block, crate::contract_state::Error> {
        let start_time = Instant::now();

        let result = self
            .executor
            .lock()
            .await
            .execute_state_transition(CallParams {
                round: ru.round,
                block_gas_limit: config::DEFAULT_BLOCK_GAS_LIMIT,
                generator_pubkey: ru.pubkey_bls.clone(),
            })
            .await?;

        let tx_hashes: Vec<_> =
            result.txs.iter().map(|t| t.inner.hash()).collect();
        let txs: Vec<_> = result.txs.into_iter().map(|t| t.inner).collect();
        let txroot = merkle_root(&tx_hashes[..]);

        let prev_block_hash = ru.hash();
        let blk_header = ledger::Header {
            version: 0,
            height: ru.round,
            timestamp: self.get_timestamp(ru.timestamp()) as i64,
            gas_limit: config::DEFAULT_BLOCK_GAS_LIMIT,
            prev_block_hash,
            seed,
            generator_bls_pubkey: node_data::bls::PublicKeyBytes(
                *ru.pubkey_bls.bytes(),
            ),
            state_hash: result.verification_output.state_root,
            event_hash: result.verification_output.event_hash,
            hash: [0; 32],
            cert: Certificate::default(),
            prev_block_cert: *ru.cert(),
            txroot,
            iteration,
            failed_iterations: IterationsInfo::new(failed_iterations),
        };

        // Apply a delay in block generator accordingly
        // In case EST call costs a second (assuming CONSENSUS_DELAY_MS=1000ms),
        // we should not sleep here
        if let Some(delay) = Duration::from_millis(config::CONSENSUS_DELAY_MS)
            .checked_sub(start_time.elapsed())
        {
            tokio::time::sleep(delay).await;
        }

        Ok(Block::new(blk_header, txs).expect("block should be valid"))
    }

    fn get_timestamp(&self, _prev_block_timestamp: i64) -> u64 {
        // TODO: use config.MaxBlockTime
        if let Ok(n) = SystemTime::now().duration_since(UNIX_EPOCH) {
            return n.as_secs();
        }
        0
    }
}
