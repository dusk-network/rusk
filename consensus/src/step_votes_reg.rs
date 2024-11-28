// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Attestation, IterationInfo, StepVotes};
use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::{payload, Message};
use node_data::StepName;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::commons::RoundUpdate;

#[derive(Clone)]
struct AttestationInfo {
    att: Attestation,

    quorum_reached_validation: bool,
    quorum_reached_ratification: bool,
}

impl fmt::Display for AttestationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "att_info: {:?}, validation: ({:?},{:?}), ratification: ({:?},{:?}) ",
            self.att.result,
            self.att.validation,
            self.quorum_reached_validation,
            self.att.ratification,
            self.quorum_reached_ratification
        )
    }
}

impl AttestationInfo {
    pub(crate) fn new(vote: Vote) -> Self {
        AttestationInfo {
            att: Attestation {
                result: vote.into(),
                ..Default::default()
            },
            quorum_reached_validation: false,
            quorum_reached_ratification: false,
        }
    }

    /// Set attestation stepvotes according to [step]. Store [quorum_reached] to
    /// calculate [AttestationInfo::is_ready]
    pub(crate) fn set_sv(
        &mut self,
        iter: u8,
        sv: StepVotes,
        step: StepName,
        quorum_reached: bool,
    ) {
        match step {
            StepName::Validation => {
                self.att.validation = sv;

                if quorum_reached {
                    self.quorum_reached_validation = quorum_reached;
                }
            }
            StepName::Ratification => {
                self.att.ratification = sv;

                if quorum_reached {
                    self.quorum_reached_ratification = quorum_reached;
                }
            }
            _ => {
                warn!(
                    event = "invalid add_sv",
                    iter,
                    data = format!("{}", self),
                    quorum_reached,
                    ?step
                );
                return;
            }
        }

        debug!(
            event = "add_sv",
            iter,
            data = format!("{}", self),
            quorum_reached
        );
    }

    /// Returns `true` if quorum is reached for both validation and
    /// ratification, except for NoQuorum votes where only the ratification
    /// quorum is checked
    fn is_ready(&self) -> bool {
        match self.att.result {
            RatificationResult::Fail(Vote::NoQuorum) => {
                self.quorum_reached_ratification
            }
            RatificationResult::Fail(Vote::Invalid(_)) => {
                self.quorum_reached_validation
                    && self.quorum_reached_ratification
            }
            RatificationResult::Fail(Vote::NoCandidate) => {
                self.quorum_reached_validation
                    && self.quorum_reached_ratification
            }
            RatificationResult::Success(Vote::Valid(_)) => {
                self.quorum_reached_validation
                    && self.quorum_reached_ratification
            }
            _ => false,
        }
    }
}

pub type SafeAttestationInfoRegistry = Arc<Mutex<AttInfoRegistry>>;

#[derive(Clone)]
struct IterationAtts {
    votes: HashMap<Vote, AttestationInfo>,
    generator: PublicKeyBytes,
}

impl IterationAtts {
    fn new(generator: PublicKeyBytes) -> Self {
        Self {
            votes: HashMap::new(),
            generator,
        }
    }

    fn failed(&self) -> Option<&AttestationInfo> {
        self.votes
            .values()
            .find(|c| c.is_ready() && c.att.result.failed())
    }

    fn get_or_insert(&mut self, vote: &Vote) -> &mut AttestationInfo {
        if !self.votes.contains_key(vote) {
            self.votes.insert(*vote, AttestationInfo::new(*vote));
        }
        self.votes.get_mut(vote).expect("Vote to be inserted")
    }
}

pub struct AttInfoRegistry {
    ru: RoundUpdate,

    /// Iterations attestations for current round keyed by iteration
    att_list: HashMap<u8, IterationAtts>,
}

impl AttInfoRegistry {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            att_list: HashMap::new(),
        }
    }

    /// Set step votes per iteration
    /// Returns a quorum if both validation and ratification for an iteration
    /// exist
    pub(crate) fn set_step_votes(
        &mut self,
        iteration: u8,
        vote: &Vote,
        sv: StepVotes,
        step: StepName,
        quorum_reached: bool,
        generator: &PublicKeyBytes,
    ) -> Option<Message> {
        if sv == StepVotes::default() {
            return None;
        }

        let iter_atts = self.get_iteration_atts(iteration, generator);
        let att_info = iter_atts.get_or_insert(vote);

        att_info.set_sv(iteration, sv, step, quorum_reached);

        let attestation = att_info.att;
        let is_ready = att_info.is_ready();

        if is_ready {
            return Some(Self::build_quorum_msg(
                &self.ru,
                iteration,
                attestation,
            ));
        }

        None
    }

    fn get_iteration_atts(
        &mut self,
        iteration: u8,
        generator: &PublicKeyBytes,
    ) -> &mut IterationAtts {
        self.att_list
            .entry(iteration)
            .or_insert_with(|| IterationAtts::new(*generator))
    }

    pub(crate) fn set_attestation(
        &mut self,
        iteration: u8,
        attestation: Attestation,
        generator: &PublicKeyBytes,
    ) {
        let iter_atts = self.get_iteration_atts(iteration, generator);

        let vote = attestation.result.vote();
        let att_info = iter_atts.get_or_insert(vote);

        // If RatificationResult is NoQuorum, we assume Validation votes did not
        // reach a quorum
        let validation_quorum = !matches!(vote, Vote::NoQuorum);

        att_info.set_sv(
            iteration,
            attestation.validation,
            StepName::Validation,
            validation_quorum,
        );
        att_info.set_sv(
            iteration,
            attestation.ratification,
            StepName::Ratification,
            true,
        );
    }

    fn build_quorum_msg(
        ru: &RoundUpdate,
        iteration: u8,
        att: Attestation,
    ) -> Message {
        let header = node_data::message::ConsensusHeader {
            prev_block_hash: ru.hash(),
            round: ru.round,
            iteration,
        };

        let payload = payload::Quorum { header, att };

        payload.into()
    }

    pub(crate) fn get_failed_atts(&self, to: u8) -> Vec<Option<IterationInfo>> {
        let mut res = Vec::with_capacity(to as usize);

        for iteration in 0u8..to {
            res.push(
                self.att_list
                    .get(&iteration)
                    .and_then(|iter| {
                        iter.failed().map(|ci| (ci, iter.generator))
                    })
                    .filter(|(ci, _)| ci.is_ready())
                    .map(|(ci, pk)| (ci.att, pk)),
            );
        }

        res
    }

    pub(crate) fn get_fail_att(&self, iteration: u8) -> Option<Attestation> {
        self.att_list
            .get(&iteration)
            .and_then(|atts| atts.failed())
            .filter(|info| info.is_ready())
            .map(|info| info.att)
    }
}
