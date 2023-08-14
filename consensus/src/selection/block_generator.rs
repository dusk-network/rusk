// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use crate::contract_state::CallParams;
use node_data::ledger::{to_str, Block, Certificate, Seed};

use crate::config;
use crate::contract_state::Operations;
use crate::merkle::merkle_root;

use dusk_bytes::Serializable;
use node_data::bls::PublicKey;
use node_data::ledger;
use node_data::message::payload::NewBlock;
use node_data::message::{Header, Message, Topics};
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
        let iteration = step / 3 + 1;
        // Sign seed
        let seed = ru
            .secret_key
            .sign(ru.pubkey_bls.inner(), &ru.seed.inner()[..])
            .to_bytes();

        let candidate = self
            .generate_block(
                &ru.pubkey_bls,
                ru.round,
                Seed::from(seed),
                ru.hash,
                ru.timestamp,
                iteration,
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

        tracing::info!(
            event = "gen_candidate",
            hash = &to_str(&candidate.header.hash),
            state_hash = &to_str(&candidate.header.state_hash),
        );

        tracing::debug!("block: {:?}", &candidate);

        Ok(Message::new_newblock(
            msg_header,
            NewBlock {
                prev_hash: ru.hash,
                candidate,
                signed_hash,
            },
        ))
    }

    async fn generate_block(
        &self,
        pubkey: &PublicKey,
        round: u64,
        seed: Seed,
        prev_block_hash: [u8; 32],
        prev_block_timestamp: i64,
        iteration: u8,
    ) -> Result<Block, crate::contract_state::Error> {
        // Delay next iteration execution so we avoid consensus-split situation.
        tokio::time::sleep(Duration::from_millis(config::CONSENSUS_DELAY_MS))
            .await;

        let result = self
            .executor
            .lock()
            .await
            .execute_state_transition(CallParams {
                round,
                block_gas_limit: config::DEFAULT_BLOCK_GAS_LIMIT,
                generator_pubkey: pubkey.clone(),
            })
            .await?;

        let tx_hashes: Vec<_> =
            result.txs.iter().map(|t| t.inner.hash()).collect();
        let txs: Vec<_> = result.txs.into_iter().map(|t| t.inner).collect();
        let txroot = merkle_root(&tx_hashes[..]);

        let blk_header = ledger::Header {
            version: 0,
            height: round,
            timestamp: self.get_timestamp(prev_block_timestamp) as i64,
            gas_limit: config::DEFAULT_BLOCK_GAS_LIMIT,
            prev_block_hash,
            seed,
            generator_bls_pubkey: node_data::bls::PublicKeyBytes(
                *pubkey.bytes(),
            ),
            state_hash: result.verification_output.state_root,
            event_hash: result.verification_output.event_hash,
            hash: [0; 32],
            cert: Certificate::default(),
            txroot,
            iteration,
        };

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
