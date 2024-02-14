// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Certificate, IterationInfo, StepVotes};
use node_data::message::payload::Vote;
use node_data::message::{payload, Message};
use node_data::StepName;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

#[derive(Clone)]
struct CertificateInfo {
    cert: Certificate,

    quorum_reached_validation: bool,
    quorum_reached_ratification: bool,
}

impl fmt::Display for CertificateInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cert_info: {:?}, validation: ({:?},{:?}), ratification: ({:?},{:?}) ",
            self.cert.result,
            self.cert.validation,
            self.quorum_reached_validation,
            self.cert.ratification,
            self.quorum_reached_ratification
        )
    }
}

impl CertificateInfo {
    pub(crate) fn new(vote: Vote) -> Self {
        CertificateInfo {
            cert: Certificate {
                result: vote.into(),
                ..Default::default()
            },
            quorum_reached_validation: false,
            quorum_reached_ratification: false,
        }
    }

    /// Set certificate stepvotes according to [step]. Store [quorum_reached] to
    /// calculate [CertificateInfo::is_ready]
    pub(crate) fn set_sv(
        &mut self,
        iter: u8,
        sv: StepVotes,
        step: StepName,
        quorum_reached: bool,
    ) {
        match step {
            StepName::Validation => {
                self.cert.validation = sv;

                if quorum_reached {
                    self.quorum_reached_validation = quorum_reached;
                }
            }
            StepName::Ratification => {
                self.cert.ratification = sv;

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

    /// Returns `true` if all fields are non-empty and quorum is reached for
    /// both validation and ratification
    fn is_ready(&self) -> bool {
        self.has_votes()
            && self.quorum_reached_validation
            && self.quorum_reached_ratification
    }

    /// Returns `true` if the certificate contains at least one vote
    fn has_votes(&self) -> bool {
        !self.cert.validation.is_empty() && !self.cert.ratification.is_empty()
    }
}

pub type SafeCertificateInfoRegistry = Arc<Mutex<CertInfoRegistry>>;

#[derive(Clone)]
struct IterationCerts {
    votes: HashMap<Vote, CertificateInfo>,
    generator: PublicKeyBytes,
}

impl IterationCerts {
    fn new(generator: PublicKeyBytes) -> Self {
        Self {
            votes: HashMap::new(),
            generator,
        }
    }

    fn get_or_insert(&mut self, vote: &Vote) -> &mut CertificateInfo {
        if !self.votes.contains_key(vote) {
            self.votes.insert(*vote, CertificateInfo::new(*vote));
        }
        self.votes.get_mut(vote).expect("Vote to be inserted")
    }
}

pub struct CertInfoRegistry {
    ru: RoundUpdate,

    /// Iterations certificates for current round keyed by iteration
    cert_list: HashMap<u8, IterationCerts>,
}

impl CertInfoRegistry {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            cert_list: HashMap::new(),
        }
    }

    /// Adds step votes per iteration
    /// Returns a quorum if both validation and ratification for an iteration
    /// exist
    pub(crate) fn add_step_votes(
        &mut self,
        iteration: u8,
        vote: &Vote,
        sv: StepVotes,
        step: StepName,
        quorum_reached: bool,
        generator: &PublicKeyBytes,
    ) -> Option<Message> {
        let cert = self
            .cert_list
            .entry(iteration)
            .or_insert_with(|| IterationCerts::new(*generator));

        let cert_info = cert.get_or_insert(vote);

        cert_info.set_sv(iteration, sv, step, quorum_reached);
        cert_info.is_ready().then(|| {
            Self::build_quorum_msg(&self.ru, iteration, cert_info.cert)
        })
    }

    fn build_quorum_msg(
        ru: &RoundUpdate,
        iteration: u8,
        cert: Certificate,
    ) -> Message {
        let header = node_data::message::ConsensusHeader {
            prev_block_hash: ru.hash(),
            round: ru.round,
            iteration,
        };

        let payload = payload::Quorum { header, cert };

        Message::new_quorum(payload)
    }

    pub(crate) fn get_nil_certificates(
        &self,
        to: u8,
    ) -> Vec<Option<IterationInfo>> {
        let mut res = Vec::with_capacity(to as usize);

        for iteration in 0u8..to {
            res.push(
                self.cert_list
                    .get(&iteration)
                    .and_then(|iter| {
                        iter.votes
                            .get(&Vote::NoCandidate)
                            .map(|ci| (ci, iter.generator))
                    })
                    .filter(|(ci, _)| ci.is_ready())
                    .map(|(ci, pk)| (ci.cert, pk)),
            );
        }

        res
    }
}
