// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::user::committee::Committee;
use async_trait::async_trait;
use node_data::message::{Message, Payload};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct Selection<D: Database> {
    pub(crate) db: Arc<Mutex<D>>,
}

#[async_trait]
impl<D: Database> MsgHandler<Message> for Selection<D> {
    /// Verifies if msg is a valid new_block message.
    fn verify(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        self.verify_new_block(&msg)?;

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

            return Ok(HandleMsgOutput::FinalResult(msg));
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::FinalResult(Message::empty()))
    }
}

impl<D: Database> Selection<D> {
    fn verify_new_block(&self, msg: &Message) -> Result<(), ConsensusError> {
        //  Verify new_block msg signature
        if let Payload::NewBlock(p) = &msg.payload {
            if msg.header.verify_signature(&p.signed_hash).is_err() {
                return Err(ConsensusError::InvalidSignature);
            }

            // TODO: Verify newblock candidate
            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }
}
