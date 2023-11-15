// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{IterCounter, RoundUpdate, StepName};
use crate::config::CONSENSUS_MAX_ITER;
use node_data::ledger::StepVotes;
use node_data::ledger::{to_str, Certificate};
use node_data::message::{payload, Message, Topics};
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

pub(crate) enum SvType {
    FirstReduction,
    SecondReduction,
}

#[derive(Default, Copy, Clone)]
struct SvEntry {
    // represents candidate block hash
    hash: Option<[u8; 32]>,
    first_red_sv: StepVotes,
    second_red_sv: StepVotes,
}

impl fmt::Display for SvEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hash = self.hash.unwrap_or_default();

        write!(
            f,
            "SvEntry: hash: {}, 1st_red: {:?}, 2nd_red: {:?}",
            to_str(&hash),
            self.first_red_sv,
            self.second_red_sv,
        )
    }
}

impl SvEntry {
    pub(crate) fn add_sv(
        &mut self,
        iter: u8,
        hash: [u8; 32],
        sv: StepVotes,
        svt: SvType,
    ) -> bool {
        if let Some(h) = self.hash {
            if h != hash {
                // Only one hash can be registered per a single iteration
                error!(desc = "multiple candidates per iter");
                return false;
            }
        } else {
            self.hash = Some(hash);
        }

        match svt {
            SvType::FirstReduction => self.first_red_sv = sv,
            SvType::SecondReduction => self.second_red_sv = sv,
        }

        debug!(event = "add_sv", iter, data = format!("{}", self));
        self.is_ready()
    }

    fn is_ready(&self) -> bool {
        !self.second_red_sv.is_empty()
            && !self.first_red_sv.is_empty()
            && self.hash.is_some()
    }

    fn is_nil(&self) -> bool {
        if let Some(hash) = self.hash {
            if hash == [0u8; 32] {
                return true;
            }
        }

        false
    }

    fn convert_to_cert(&self) -> Certificate {
        Certificate {
            first_reduction: self.first_red_sv,
            second_reduction: self.second_red_sv,
        }
    }
}

pub type SafeStepVotesRegistry = Arc<Mutex<StepVotesRegistry>>;

pub struct StepVotesRegistry {
    ru: RoundUpdate,
    sv_table: [SvEntry; CONSENSUS_MAX_ITER as usize],
}

impl StepVotesRegistry {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            sv_table: [SvEntry::default(); CONSENSUS_MAX_ITER as usize],
        }
    }

    /// Adds step votes per iteration
    /// Returns an agreement if both reductions for an iteration are available
    pub(crate) fn add_step_votes(
        &mut self,
        step: u8,
        hash: [u8; 32],
        sv: StepVotes,
        svt: SvType,
    ) -> Option<Message> {
        let iter_num = u8::from_step(step);
        if iter_num as usize >= self.sv_table.len() {
            return None;
        }

        let r = &mut self.sv_table[iter_num as usize];
        if r.add_sv(iter_num, hash, sv, svt) {
            return Some(Self::build_agreement_msg(
                self.ru.clone(),
                iter_num,
                *r,
            ));
        }

        None
    }

    fn build_agreement_msg(
        ru: RoundUpdate,
        iteration: u8,
        result: SvEntry,
    ) -> Message {
        let hdr = node_data::message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            step: iteration.step_from_name(StepName::SecondRed),
            block_hash: result.hash.unwrap_or_default(),
            topic: Topics::Agreement as u8,
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        let payload = payload::Agreement {
            signature,
            first_step: result.first_red_sv,
            second_step: result.second_red_sv,
        };

        Message::new_agreement(hdr, payload)
    }

    pub(crate) fn get_nil_certificates(
        &mut self,
        from: usize,
        to: usize,
    ) -> Vec<Option<Certificate>> {
        let to = std::cmp::min(to, self.sv_table.len());
        let mut res = Vec::with_capacity(to - from);

        for item in &self.sv_table[from..to] {
            if item.is_nil() {
                res.push(Some(item.convert_to_cert()));
            } else {
                res.push(None)
            }
        }

        res
    }
}
