// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{Block, Certificate, RoundUpdate, Topics};
use crate::contract_state::Operations;
use crate::messages::payload::NewBlock;
use crate::messages::{Header, Message};
use crate::util::pubkey::ConsensusPublicKey;
use crate::{commons, config};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

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
    ) -> Result<Message, crate::contract_state::Error> {
        let candidate = self
            .generate_block(
                &ru.pubkey_bls,
                ru.round,
                ru.seed,
                ru.hash,
                ru.timestamp,
            )
            .await?;

        let msg_header = Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            block_hash: candidate.header.hash,
            step,
            topic: Topics::NewBlock as u8,
        };

        let signed_hash =
            msg_header.sign(&ru.secret_key, ru.pubkey_bls.inner());

        Ok(Message::from_newblock(
            msg_header,
            NewBlock {
                prev_hash: [0; 32],
                candidate,
                signed_hash,
            },
        ))
    }

    async fn generate_block(
        &self,
        pubkey: &ConsensusPublicKey,
        round: u64,
        seed: [u8; 32],
        prev_block_hash: [u8; 32],
        prev_block_timestamp: i64,
    ) -> Result<Block, crate::contract_state::Error> {
        // TODO: fetch mempool transactions

        // Delay next iteration execution so we avoid consensus-split situation.
        tokio::time::sleep(Duration::from_millis(config::CONSENSUS_DELAY_MS))
            .await;

        self.executor.lock().await.execute_state_transition(
            crate::contract_state::CallParams::default(),
        )?;

        let blk_header = commons::Header {
            version: 0,
            height: round,
            timestamp: self.get_timestamp(prev_block_timestamp) as i64,
            gas_limit: 0,
            prev_block_hash,
            seed,
            generator_bls_pubkey: *pubkey.bytes(),
            state_hash: [0; 32],
            hash: [0; 32],
            cert: Certificate::default(),
        };

        Ok(Block::new(blk_header, vec![]).expect("block should be valid"))
    }

    fn get_timestamp(&self, _prev_block_timestamp: i64) -> u64 {
        // TODO: use config.MaxBlockTime
        if let Ok(n) = SystemTime::now().duration_since(UNIX_EPOCH) {
            return n.as_secs();
        }
        0
    }
}
