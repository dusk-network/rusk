// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.

use node_data::ledger::*;

use std::time::Duration;

use execution_core::signatures::bls::SecretKey as BlsSecretKey;
use node_data::bls::PublicKey;
use node_data::message::{AsyncQueue, Message, Payload};

use crate::operations::Voter;

#[derive(Clone, Default, Debug)]
pub struct RoundUpdate {
    // Current round number of the ongoing consensus
    pub round: u64,

    // This provisioner consensus keys
    pub pubkey_bls: PublicKey,
    pub secret_key: BlsSecretKey,

    seed: Seed,
    hash: [u8; 32],
    att: Attestation,
    att_voters: Vec<Voter>,
    timestamp: u64,

    pub base_timeout: Duration,
}

impl RoundUpdate {
    pub fn new(
        pubkey_bls: PublicKey,
        secret_key: BlsSecretKey,
        tip_header: &Header,
        base_timeout: Duration,
        att_voters: Vec<Voter>,
    ) -> Self {
        let round = tip_header.height + 1;
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            att: tip_header.att,
            hash: tip_header.hash,
            seed: tip_header.seed,
            timestamp: tip_header.timestamp,
            base_timeout,
            att_voters,
        }
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    pub fn att(&self) -> &Attestation {
        &self.att
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn att_voters(&self) -> &Vec<Voter> {
        &self.att_voters
    }
}

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn store_candidate_block(&mut self, b: Block);
    async fn get_last_iter(&self) -> (Hash, u8);
    async fn store_last_iter(&mut self, data: (Hash, u8));
}

#[derive(Clone)]
pub(crate) struct QuorumMsgSender {
    queue: AsyncQueue<Message>,
}

impl QuorumMsgSender {
    pub(crate) fn new(queue: AsyncQueue<Message>) -> Self {
        Self { queue }
    }

    /// Sends an quorum (internally) to the lower layer.
    pub(crate) async fn send_quorum(&self, msg: Message) {
        match &msg.payload {
            Payload::Quorum(q) if !q.att.ratification.is_empty() => {
                tracing::debug!(
                    event = "send quorum_msg",
                    vote = ?q.vote(),
                    round = msg.header.round,
                    iteration = msg.header.iteration,
                    validation = ?q.att.validation,
                    ratification = ?q.att.ratification,
                );
            }
            _ => return,
        }

        self.queue.try_send(msg);
    }
}
