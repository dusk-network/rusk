// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{Database, RoundUpdate};
use crate::config::{
    is_emergency_iter, MAX_BLOCK_SIZE, MAX_NUMBER_OF_FAULTS,
    MAX_NUMBER_OF_TRANSACTIONS,
};
use crate::errors::ConsensusError;
use crate::merkle::merkle_root;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::user::committee::Committee;
use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::to_str;
use node_data::message::payload::{Candidate, GetResource, Inv};
use tracing::info;

use crate::iteration_ctx::RoundCommittees;
use node_data::message::{
    Message, Payload, SignedStepMessage, StepMessage, WireMessage,
};
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
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        let iteration = p.header().iteration;
        let generator = round_committees
            .get_generator(iteration)
            .expect("committee to be created before run");
        super::handler::verify_new_block(p, &generator)?;

        Ok(())
    }

    /// Collects а new_block message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _committee: &Committee,
        _generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        // store candidate block
        let p = Self::unwrap_msg(&msg)?;
        self.db
            .lock()
            .await
            .store_candidate_block(p.candidate.clone())
            .await;

        Ok(HandleMsgOutput::Ready(msg))
    }

    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _committee: &Committee,
        _generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let p = Self::unwrap_msg(&msg)?;

        self.db
            .lock()
            .await
            .store_candidate_block(p.candidate.clone())
            .await;

        Ok(HandleMsgOutput::Ready(msg))
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message> {
        if is_emergency_iter(curr_iteration) {
            // While we are in Emergency mode but still the candidate is missing
            // then we should request it
            info!(
                event = "request candidate block",
                src = "emergency_iter",
                iteration = curr_iteration,
                prev_block_hash = to_str(&ru.hash())
            );

            let mut inv = Inv::new(1);
            inv.add_candidate_from_iteration(ru.hash(), curr_iteration);
            let msg = GetResource::new(inv, None, u64::MAX, 0);
            return Some(msg.into());
        }

        None
    }
}

impl<D: Database> ProposalHandler<D> {
    pub(crate) fn new(db: Arc<Mutex<D>>) -> Self {
        Self { db }
    }

    fn unwrap_msg(msg: &Message) -> Result<&Candidate, ConsensusError> {
        match &msg.payload {
            Payload::Candidate(c) => Ok(c),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }
}

fn verify_new_block(
    p: &Candidate,
    expected_generator: &PublicKeyBytes,
) -> Result<(), ConsensusError> {
    if expected_generator != p.sign_info().signer.bytes() {
        return Err(ConsensusError::NotCommitteeMember);
    }

    let candidate_size = p
        .candidate
        .size()
        .map_err(|_| ConsensusError::UnknownBlockSize)?;
    if candidate_size > MAX_BLOCK_SIZE {
        return Err(ConsensusError::InvalidBlockSize(candidate_size));
    }

    //  Verify new_block msg signature
    p.verify_signature()?;

    if p.consensus_header().prev_block_hash
        != p.candidate.header().prev_block_hash
    {
        return Err(ConsensusError::InvalidBlockHash);
    }

    if p.candidate.txs().len() > MAX_NUMBER_OF_TRANSACTIONS {
        return Err(ConsensusError::TooManyTransactions(
            p.candidate.txs().len(),
        ));
    }

    let tx_hashes: Vec<_> =
        p.candidate.txs().iter().map(|t| t.hash()).collect();
    let tx_root = merkle_root(&tx_hashes[..]);
    if tx_root != p.candidate.header().txroot {
        return Err(ConsensusError::InvalidBlock);
    }

    if p.candidate.faults().len() > MAX_NUMBER_OF_FAULTS {
        return Err(ConsensusError::TooManyFaults(p.candidate.faults().len()));
    }

    let fault_hashes: Vec<_> =
        p.candidate.faults().iter().map(|t| t.hash()).collect();
    let fault_root = merkle_root(&fault_hashes[..]);
    if fault_root != p.candidate.header().faultroot {
        return Err(ConsensusError::InvalidBlock);
    }

    Ok(())
}

pub fn verify_stateless(
    c: &Candidate,
    round_committees: &RoundCommittees,
) -> Result<(), ConsensusError> {
    let iteration = c.header().iteration;
    let generator = round_committees
        .get_generator(iteration)
        .expect("committee to be created before run");
    verify_new_block(c, &generator)?;

    Ok(())
}
