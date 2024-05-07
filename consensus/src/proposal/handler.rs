// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::merkle::merkle_root;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::user::committee::Committee;
use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::message::payload::Candidate;

use crate::iteration_ctx::RoundCommittees;
use node_data::message::{Message, Payload, StepMessage};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ProposalHandler<D: Database> {
    pub(crate) db: Arc<Mutex<D>>,
}

#[async_trait]
impl<D: Database> MsgHandler for ProposalHandler<D> {
    /// Verifies if msg is a valid new_block message.
    fn verify(
        &self,
        msg: &Message,
        iteration: u8,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let generator = round_committees
            .get_generator(iteration)
            .expect("committee to be created before run");
        self.verify_new_block(msg, &generator)?;

        Ok(())
    }

    /// Collects а new_block message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        // store candidate block
        let p = Self::unwrap_msg(&msg)?;
        self.db
            .lock()
            .await
            .store_candidate_block(p.candidate.clone());

        Ok(HandleMsgOutput::Ready(msg))
    }

    async fn collect_from_past(
        &mut self,
        _msg: Message,
        _ru: &RoundUpdate,
        _committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::Pending)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(&self) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::Ready(Message::empty()))
    }
}

impl<D: Database> ProposalHandler<D> {
    pub(crate) fn new(db: Arc<Mutex<D>>) -> Self {
        Self { db }
    }

    fn verify_new_block(
        &self,
        msg: &Message,
        expected_generator: &PublicKeyBytes,
    ) -> Result<(), ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        //  Verify new_block msg signature
        p.verify_signature()?;

        if msg.header.prev_block_hash != p.candidate.header().prev_block_hash {
            return Err(ConsensusError::InvalidBlockHash);
        }

        let tx_hashes: Vec<[u8; 32]> =
            p.candidate.txs().iter().map(|t| t.id()).collect();
        let tx_root = merkle_root(&tx_hashes[..]);
        if tx_root != p.candidate.header().txroot {
            return Err(ConsensusError::InvalidBlock);
        }

        if expected_generator != p.sign_info.signer.bytes() {
            return Err(ConsensusError::NotCommitteeMember);
        }

        Ok(())
    }

    fn unwrap_msg(msg: &Message) -> Result<&Candidate, ConsensusError> {
        match &msg.payload {
            Payload::Candidate(c) => Ok(c),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }
}
