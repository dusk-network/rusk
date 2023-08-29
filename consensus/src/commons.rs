// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::contract_state::Operations;
// TODO: use crate::messages::payload::StepVotes;
// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use node_data::ledger::*;
use node_data::message;
use tracing::Instrument;

use crate::contract_state::CallParams;
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::DeserializableSlice;
use node_data::bls::PublicKey;

use node_data::message::{AsyncQueue, Message};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error};

#[derive(Clone, Default, Debug)]
#[allow(unused)]
pub struct RoundUpdate {
    pub round: u64,
    pub seed: Seed,
    pub hash: [u8; 32],
    pub timestamp: i64,
    pub pubkey_bls: PublicKey,
    pub secret_key: SecretKey,
}

impl RoundUpdate {
    pub fn new(
        round: u64,
        pubkey_bls: PublicKey,
        secret_key: SecretKey,
        seed: Seed,
    ) -> Self {
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            seed,
            hash: [0u8; 32],
            timestamp: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidBlockHash,
    InvalidSignature,
    InvalidMsgType,
    FutureEvent,
    PastEvent,
    NotCommitteeMember,
    NotImplemented,
    NotReady,
    MaxStepReached,
    ChildTaskTerminated,
    Canceled,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_send_reduction<T: Operations + 'static>(
    join_set: &mut JoinSet<()>,
    verified_hash: Arc<Mutex<[u8; 32]>>,
    candidate: Block,
    pubkey: PublicKey,
    ru: RoundUpdate,
    step: u8,
    outbound: AsyncQueue<Message>,
    inbound: AsyncQueue<Message>,
    executor: Arc<Mutex<T>>,
) {
    let hash = to_str(&candidate.header.hash);

    join_set.spawn(
        async move {
            let hash = candidate.header.hash;
            let already_verified = *verified_hash.lock().await == hash;

            if !already_verified && hash != [0u8; 32] {
                let pubkey = &candidate.header.generator_bls_pubkey.0;
                let generator =
                    match dusk_bls12_381_sign::PublicKey::from_slice(pubkey) {
                        Ok(pubkey) => pubkey,
                        Err(e) => {
                            error!(
                        "unable to decode generator BLS Pubkey {}, err: {:?}",
                        hex::encode(pubkey),
                        e,
                    );
                            return;
                        }
                    };

                match executor
                    .lock()
                    .await
                    .verify_state_transition(
                        CallParams {
                            round: ru.round,
                            block_gas_limit: candidate.header.gas_limit,
                            generator_pubkey: PublicKey::new(generator),
                        },
                        candidate.txs.clone(),
                    )
                    .await
                {
                    Ok(verification_output) => {
                        // Ensure the `event_hash` and `state_root` returned
                        // from the VST call are the
                        // ones we expect to have with the
                        // current candidate block.
                        if verification_output.event_hash
                            != candidate.header.event_hash
                        {
                            error!(
                                desc = "event hash mismatch",
                                event_hash =
                                    hex::encode(verification_output.event_hash),
                                candidate_event_hash =
                                    hex::encode(candidate.header.event_hash),
                            );
                            return;
                        }

                        if verification_output.state_root
                            != candidate.header.state_hash
                        {
                            error!(
                                desc = "state hash mismatch",
                                vst_state_hash =
                                    hex::encode(verification_output.state_root),
                                state_hash =
                                    hex::encode(candidate.header.state_hash),
                            );
                            return;
                        }
                    }
                    Err(e) => {
                        error!("VST failed with err: {:?}", e);
                        return;
                    }
                };
            }

            if already_verified && hash != [0u8; 32] {
                debug!(event = "vst call skipped", reason = "already_verified",)
            }

            {
                let mut guard = verified_hash.lock().await;
                *guard = hash;
            }

            let hdr = message::Header {
                pubkey_bls: pubkey,
                round: ru.round,
                step,
                block_hash: hash,
                topic: message::Topics::Reduction as u8,
            };

            let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

            // Sign and construct reduction message
            let msg = message::Message::new_reduction(
                hdr,
                message::payload::Reduction { signature },
            );

            //   publish
            outbound.send(msg.clone()).await.unwrap_or_else(|err| {
                error!("unable to publish reduction msg {:?}", err)
            });

            // Register my vote locally
            inbound.send(msg).await.unwrap_or_else(|err| {
                error!("unable to register reduction msg {:?}", err)
            });
        }
        .instrument(tracing::info_span!("voting", hash)),
    );
}
#[async_trait::async_trait]
pub trait Database: Send + Sync {
    fn store_candidate_block(&mut self, b: Block);
    async fn get_candidate_block_by_hash(
        &self,
        h: &Hash,
    ) -> anyhow::Result<Block>;
    fn delete_candidate_blocks(&mut self);
}
