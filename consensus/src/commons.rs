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

use crate::contract_state::CallParams;
use bytes::{BufMut, BytesMut};
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::DeserializableSlice;
use node_data::bls::PublicKey;
use node_data::message::AsyncQueue;
use node_data::message::Message;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

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

pub fn marshal_signable_vote(
    round: u64,
    step: u8,
    block_hash: &[u8; 32],
) -> BytesMut {
    let mut msg = BytesMut::with_capacity(block_hash.len() + 8 + 1);
    msg.put_u64_le(round);
    msg.put_u8(step);
    msg.put(&block_hash[..]);

    msg
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_send_reduction<T: Operations + 'static>(
    candidate: Block,
    pubkey: PublicKey,
    ru: RoundUpdate,
    step: u8,
    outbound: AsyncQueue<Message>,
    inbound: AsyncQueue<Message>,
    vc_list: Arc<Mutex<HashSet<[u8; 32]>>>,
    executor: Arc<Mutex<T>>,
) {
    tokio::spawn(async move {
        if candidate == Block::default() {
            return;
        }

        let hash = candidate.header.hash;
        let already_verified = vc_list.lock().await.contains(&hash);

        if !already_verified {
            let pubkey = &candidate.header.generator_bls_pubkey.0;
            let generator =
                match dusk_bls12_381_sign::PublicKey::from_slice(pubkey) {
                    Ok(pubkey) => pubkey,
                    Err(e) => {
                        tracing::error!(
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
                    // Ensure the `event_hash` and `state_root` returned from
                    // the VST call are the ones we expect to have with the
                    // current candidate block.
                    if verification_output.event_hash
                        != candidate.header.event_hash
                    {
                        tracing::error!(
                            "VST failed with invalid event_hash: {}, candidate_event_hash: {}",
                            hex::encode(verification_output.event_hash),
                            hex::encode(candidate.header.event_hash),
                        );
                        return;
                    }

                    if verification_output.state_root
                        != candidate.header.state_hash
                    {
                        tracing::error!(
                            "VST failed with invalid state_hash: {}, candidate_state_hash: {}",
                            hex::encode(verification_output.state_root),
                            hex::encode(candidate.header.state_hash),
                        );
                        return;
                    }
                }
                Err(e) => {
                    tracing::error!("VST failed with err: {:?}", e);
                    return;
                }
            };
        }

        vc_list.lock().await.insert(hash);

        let hdr = message::Header {
            pubkey_bls: pubkey,
            round: ru.round,
            step,
            block_hash: hash,
            topic: message::Topics::Reduction as u8,
        };

        let signed_hash = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        // Sign and construct reduction message
        let msg = message::Message::new_reduction(
            hdr,
            message::payload::Reduction { signed_hash },
        );

        //   publish
        outbound.send(msg.clone()).await.unwrap_or_else(|err| {
            tracing::error!("unable to publish reduction msg {:?}", err)
        });

        // Register my vote locally
        inbound.send(msg).await.unwrap_or_else(|err| {
            tracing::error!("unable to register reduction msg {:?}", err)
        });
    });
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
