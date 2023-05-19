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

use bytes::{BufMut, BytesMut};
use dusk_bls12_381_sign::SecretKey;
use node_data::bls::PublicKey;
use node_data::message::AsyncQueue;
use node_data::message::Message;
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
    pub secret_key: SecretKey, // TODO: should be here?? SecretKey
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

pub fn spawn_send_reduction<T: Operations + 'static>(
    candidate: Block,
    pubkey: PublicKey,
    ru: RoundUpdate,
    step: u8,
    mut outbound: AsyncQueue<Message>,
    mut inbound: AsyncQueue<Message>,
    executor: Arc<Mutex<T>>,
) {
    tokio::spawn(async move {
        if let Err(e) = executor.lock().await.verify_state_transition(
            crate::contract_state::CallParams::default(),
        ) {
            tracing::error!("verify state transition failed with err: {:?}", e);
            return;
        }

        let hdr = message::Header {
            pubkey_bls: pubkey,
            round: ru.round,
            step,
            block_hash: candidate.header.hash,
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
