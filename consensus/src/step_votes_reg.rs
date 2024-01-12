// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use crate::config::CONSENSUS_MAX_ITER;
use node_data::ledger::StepVotes;
use node_data::ledger::{to_str, Certificate};
use node_data::message::{payload, Message, Topics};
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

pub(crate) enum SvType {
    Validation,
    Ratification,
}

#[derive(Default, Clone, Copy)]
struct CertificateInfo {
    /// represents candidate block hash
    hash: Option<[u8; 32]>,
    cert: Certificate,

    quorum_reached_validation: bool,
    quorum_reached_ratification: bool,
}

impl fmt::Display for CertificateInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hash = self.hash.unwrap_or_default();

        write!(
            f,
            "cert_info: hash: {}, validation: ({:?},{:?}), ratification: ({:?},{:?}) ",
            to_str(&hash),
            self.cert.validation,
            self.quorum_reached_validation,
            self.cert.ratification,
            self.quorum_reached_ratification
        )
    }
}

impl CertificateInfo {
    pub(crate) fn add_sv(
        &mut self,
        iter: u8,
        hash: [u8; 32],
        sv: StepVotes,
        svt: SvType,
        quorum_reached: bool,
    ) -> bool {
        if let Some(h) = self.hash {
            if h != hash {
                error!("Attempting to replace {h:?} with {hash:?}");
                return false;
            }
        } else {
            self.hash = Some(hash);
        }

        match svt {
            SvType::Validation => {
                self.cert.validation = sv;

                if quorum_reached {
                    self.quorum_reached_validation = quorum_reached;
                }
            }
            SvType::Ratification => {
                self.cert.ratification = sv;

                if quorum_reached {
                    self.quorum_reached_ratification = quorum_reached;
                }
            }
        }

        debug!(
            event = "add_sv",
            iter,
            data = format!("{}", self),
            quorum_reached
        );

        self.is_ready()
    }

    /// Returns `true` if all fields are non-empty and quorum is reached for
    /// both validation and ratification
    fn is_ready(&self) -> bool {
        !self.cert.validation.is_empty()
            && !self.cert.ratification.is_empty()
            && self.hash.is_some()
            && self.quorum_reached_validation
            && self.quorum_reached_ratification
    }

    /// Returns `true` if the certificate has empty hash
    fn is_nil(&self) -> bool {
        self.hash.map(|h| h == [0u8; 32]).unwrap_or_default()
    }
}

pub type SafeCertificateInfoRegistry = Arc<Mutex<CertInfoRegistry>>;

pub struct CertInfoRegistry {
    ru: RoundUpdate,

    /// List of iterations certificates. Position in the array represents
    /// iteration number.
    cert_list: [CertificateInfo; CONSENSUS_MAX_ITER as usize],
}

impl CertInfoRegistry {
    pub(crate) fn new(ru: RoundUpdate) -> Self {
        Self {
            ru,
            cert_list: [CertificateInfo::default();
                CONSENSUS_MAX_ITER as usize],
        }
    }

    /// Adds step votes per iteration
    /// Returns a quorum if both validation and ratification for an iteration
    /// exist
    pub(crate) fn add_step_votes(
        &mut self,
        iteration: u8,
        hash: [u8; 32],
        sv: StepVotes,
        svt: SvType,
        quorum_reached: bool,
    ) -> Option<Message> {
        if iteration as usize >= self.cert_list.len() {
            return None;
        }

        let r = &mut self.cert_list[iteration as usize];
        if r.add_sv(iteration, hash, sv, svt, quorum_reached) {
            return Some(Self::build_quorum_msg(
                self.ru.clone(),
                iteration,
                *r,
            ));
        }

        None
    }

    fn build_quorum_msg(
        ru: RoundUpdate,
        iteration: u8,
        result: CertificateInfo,
    ) -> Message {
        let hdr = node_data::message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            iteration,
            block_hash: result.hash.unwrap_or_default(),
            topic: Topics::Quorum,
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        let payload = payload::Quorum {
            signature,
            validation: result.cert.validation,
            ratification: result.cert.ratification,
        };

        Message::new_quorum(hdr, payload)
    }

    pub(crate) fn get_nil_certificates(
        &mut self,
        from: usize,
        to: usize,
    ) -> Vec<Option<Certificate>> {
        let to = std::cmp::min(to, self.cert_list.len());
        let mut res = Vec::with_capacity(to - from);

        for item in &self.cert_list[from..to] {
            if item.is_nil() {
                res.push(Some(item.cert));
            } else {
                res.push(None)
            }
        }

        res
    }
}
