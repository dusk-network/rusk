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
use tracing::debug;

pub(crate) enum SvType {
    FirstReduction,
    SecondReduction,
}

#[derive(Default, Clone, Copy)]
struct AgreementInfo {
    /// represents candidate block hash
    hash: Option<[u8; 32]>,
    cert: Certificate,
}

impl fmt::Display for AgreementInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hash = self.hash.unwrap_or_default();

        write!(
            f,
            "agreement_info: hash: {}, 1st_red: {:?}, 2nd_red: {:?}",
            to_str(&hash),
            self.cert.first_reduction,
            self.cert.second_reduction,
        )
    }
}

impl AgreementInfo {
    pub(crate) fn add_sv(
        &mut self,
        iter: u8,
        hash: [u8; 32],
        sv: StepVotes,
        svt: SvType,
    ) -> bool {
        if let Some(h) = self.hash {
            if h != hash {
                return false;
            }
        } else {
            self.hash = Some(hash);
        }

        match svt {
            SvType::FirstReduction => self.cert.first_reduction = sv,
            SvType::SecondReduction => self.cert.second_reduction = sv,
        }

        debug!(event = "add_sv", iter, data = format!("{}", self));
        self.is_ready()
    }

    /// Returns `true` if all fields are non-empty
    fn is_ready(&self) -> bool {
        !self.cert.first_reduction.is_empty()
            && !self.cert.second_reduction.is_empty()
            && self.hash.is_some()
    }

    /// Returns `true` if a nil_agreement is ready
    /// NB: Nil agreement is an agreement for NIL hash
    fn is_nil(&self) -> bool {
        if let Some(hash) = self.hash {
            if hash == [0u8; 32] {
                return true;
            }
        }

        false
    }
}

pub type SafeAgreementInfoRegistry = Arc<Mutex<AgreementInfoRegistry>>;

pub struct AgreementInfoRegistry {
    ru: RoundUpdate,

    /// List of iterations agreements. Position in the array represents
    /// iteration number.
    agreement_reg: [AgreementInfo; CONSENSUS_MAX_ITER as usize],
}

impl AgreementInfoRegistry {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            agreement_reg: [AgreementInfo::default();
                CONSENSUS_MAX_ITER as usize],
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
        if iter_num as usize >= self.agreement_reg.len() {
            return None;
        }

        let r = &mut self.agreement_reg[iter_num as usize];
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
        result: AgreementInfo,
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
            first_step: result.cert.first_reduction,
            second_step: result.cert.second_reduction,
        };

        Message::new_agreement(hdr, payload)
    }

    pub(crate) fn get_nil_certificates(
        &mut self,
        from: usize,
        to: usize,
    ) -> Vec<Option<Certificate>> {
        let to = std::cmp::min(to, self.agreement_reg.len());
        let mut res = Vec::with_capacity(to - from);

        for item in &self.agreement_reg[from..to] {
            if item.is_nil() {
                res.push(Some(item.cert));
            } else {
                res.push(None)
            }
        }

        res
    }
}
