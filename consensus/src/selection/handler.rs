// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::merkle::merkle_root;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::SafeStepVotesRegistry;
use crate::user::committee::Committee;
use async_trait::async_trait;

use node_data::message::{Message, Payload};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Selection<D: Database> {
    pub(crate) db: Arc<Mutex<D>>,
    pub(crate) _sv_registry: SafeStepVotesRegistry,
}

#[async_trait]
impl<D: Database> MsgHandler<Message> for Selection<D> {
    /// Verifies if msg is a valid new_block message.
    fn verify(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        self.verify_new_block(&msg, committee)?;

        Ok(msg)
    }

    /// Collects а new_block message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        // store candidate block
        if let Payload::NewBlock(p) = &msg.payload {
            self.db
                .lock()
                .await
                .store_candidate_block(p.candidate.clone());

            return Ok(HandleMsgOutput::Ready(msg));
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::Ready(Message::empty()))
    }
}

impl<D: Database> Selection<D> {
    pub(crate) fn new(
        db: Arc<Mutex<D>>,
        sv_registry: SafeStepVotesRegistry,
    ) -> Self {
        Self {
            db,
            _sv_registry: sv_registry,
        }
    }

    fn verify_new_block(
        &self,
        msg: &Message,
        committee: &Committee,
    ) -> Result<(), ConsensusError> {
        if let Payload::NewBlock(p) = &msg.payload {
            //  Verify new_block msg signature
            if msg.header.verify_signature(&p.signature).is_err() {
                return Err(ConsensusError::InvalidSignature);
            }

            if msg.header.block_hash != p.candidate.header().hash {
                return Err(ConsensusError::InvalidBlockHash);
            }

            let tx_hashes: Vec<[u8; 32]> =
                p.candidate.txs().iter().map(|t| t.hash()).collect();
            let tx_root = merkle_root(&tx_hashes[..]);
            if tx_root != p.candidate.header().txroot {
                return Err(ConsensusError::InvalidBlock);
            }

            if !committee.is_member(&msg.header.pubkey_bls) {
                return Err(ConsensusError::NotCommitteeMember);
            }

            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }
}
