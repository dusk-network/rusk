// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use crate::config::CONSENSUS_MAX_ITER;
use node_data::ledger::to_str;
use node_data::ledger::StepVotes;
use node_data::message::{payload, Message, Topics};
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

#[derive(Default, Copy, Clone)]
struct IterationResult {
    pub(crate) candidate_hash: [u8; 32],
    pub(crate) first_red_sv: StepVotes,
    pub(crate) sec_red_sv: StepVotes,
}

impl fmt::Display for IterationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IterationResult: hash: {}, 1st_red: {:?}, 2nd_red: {:?}",
            to_str(&self.candidate_hash),
            self.first_red_sv,
            self.sec_red_sv,
        )
    }
}

impl IterationResult {
    pub(crate) fn add_step_votes(
        &mut self,
        iter: u8,
        hash: [u8; 32],
        sv: StepVotes,
        is_1st_reduction: bool,
    ) -> bool {
        if self.candidate_hash == [0u8; 32] {
            self.candidate_hash = hash;
        } else if self.candidate_hash != hash {
            // More than one hash per an iteration
            error!(desc = "multiple candidates per iter");
            return false;
        }

        if is_1st_reduction {
            self.first_red_sv = sv;
        } else {
            self.sec_red_sv = sv;
        }

        debug!(event = "add_sv", iter, data = format!("{}", self));

        self.is_ready()
    }

    fn is_ready(&self) -> bool {
        !self.sec_red_sv.is_empty()
            && !self.first_red_sv.is_empty()
            && self.candidate_hash != [0u8; 32]
    }
}

pub(crate) type SafeRoundCtx = Arc<Mutex<RoundCtx>>;

pub(crate) struct RoundCtx {
    ru: RoundUpdate,
    result_table: [IterationResult; CONSENSUS_MAX_ITER as usize],
}

impl RoundCtx {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            result_table: [IterationResult::default();
                CONSENSUS_MAX_ITER as usize],
        }
    }
    pub(crate) fn add_step_votes(
        &mut self,
        step: u8,
        hash: [u8; 32],
        sv: StepVotes,
        is_1st_reduction: bool,
    ) -> Option<Message> {
        let iteration = ((step - 1) / 3 + 1) as usize;

        self.result_table[iteration].add_step_votes(
            iteration as u8,
            hash,
            sv,
            is_1st_reduction,
        );

        let r = &self.result_table[iteration];

        if r.is_ready() {
            return Some(self.build_agreement_msg(iteration as u8, r));
        }

        None
    }

    fn build_agreement_msg(
        &self,
        iteration: u8,
        result: &IterationResult,
    ) -> Message {
        let hdr = node_data::message::Header {
            pubkey_bls: self.ru.pubkey_bls.clone(),
            round: self.ru.round,
            step: (iteration - 1) * 3 + 3,
            block_hash: result.candidate_hash,
            topic: Topics::Agreement as u8,
        };

        let signature =
            hdr.sign(&self.ru.secret_key, self.ru.pubkey_bls.inner()); // TODO: this should be deleted

        let payload = payload::Agreement {
            signature,
            first_step: result.first_red_sv,
            second_step: result.sec_red_sv,
        };

        Message::new_agreement(hdr, payload)
    }
}
