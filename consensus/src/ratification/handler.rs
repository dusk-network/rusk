// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::step_votes_reg::{SafeCertificateInfoRegistry, SvType};
use async_trait::async_trait;
use node_data::ledger::Hash;
use node_data::{ledger, StepName};
use tracing::{error, warn};

use crate::aggregator::Aggregator;

use crate::config;
use crate::execution_ctx::RoundCommittees;
use crate::quorum::verifiers::verify_votes;
use node_data::message::payload::{QuorumType, Ratification, ValidationResult};
use node_data::message::{payload, Message, Payload, Topics};

use crate::user::committee::Committee;
use crate::user::sortition;

pub struct RatificationHandler {
    pub(crate) sv_registry: SafeCertificateInfoRegistry,

    pub(crate) aggregator: Aggregator,
    pub(crate) validation_result: ValidationResult,
    pub(crate) curr_iteration: u8,
}

#[async_trait]
impl MsgHandler<Message> for RatificationHandler {
    fn verify(
        &self,
        msg: &Message,
        ru: &RoundUpdate,
        iteration: u8,
        _committee: &Committee,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        if let Payload::Ratification(p) = &msg.payload {
            if msg.header.verify_signature(&p.signature).is_err() {
                return Err(ConsensusError::InvalidSignature);
            }

            Self::verify_validation_result(
                ru,
                iteration,
                round_committees,
                &p.validation_result,
            )?;

            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Collect the ratification message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let iteration = msg.header.iteration;
        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            warn!(
                event = "drop message",
                reason = "invalid iteration number",
                msg_iteration = iteration,
            );
            return Ok(HandleMsgOutput::Pending(msg));
        }

        let ratification = Self::unwrap_msg(&msg)?;

        // Collect vote, if msg payload is of ratification type
        if let Some((block_hash, ratification_sv, quorum_reached)) = self
            .aggregator
            .collect_vote(committee, &msg.header, &ratification.signature)
        {
            // Record any signature in global registry
            _ = self.sv_registry.lock().await.add_step_votes(
                iteration,
                block_hash,
                ratification_sv,
                SvType::Ratification,
                quorum_reached,
            );

            if quorum_reached {
                return Ok(HandleMsgOutput::Ready(self.build_quorum_msg(
                    ru,
                    iteration,
                    block_hash,
                    ratification.validation_result.sv,
                    ratification_sv,
                )));
            }
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Collects the reduction message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        iteration: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let ratification = Self::unwrap_msg(&msg)?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv, quorum_reached)) = self.aggregator.collect_vote(
            committee,
            &msg.header,
            &ratification.signature,
        ) {
            // Record any signature in global registry
            if let Some(quorum_msg) =
                self.sv_registry.lock().await.add_step_votes(
                    iteration,
                    hash,
                    sv,
                    SvType::Ratification,
                    quorum_reached,
                )
            {
                return Ok(HandleMsgOutput::Ready(quorum_msg));
            }
        }

        Ok(HandleMsgOutput::Pending(msg))
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _iteration: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::Ready(Message::empty()))
    }
}

impl RatificationHandler {
    pub(crate) fn new(sv_registry: SafeCertificateInfoRegistry) -> Self {
        Self {
            sv_registry,
            aggregator: Default::default(),
            validation_result: Default::default(),
            curr_iteration: 0,
        }
    }

    fn build_quorum_msg(
        &self,
        ru: &RoundUpdate,
        iteration: u8,
        block_hash: Hash,
        validation: ledger::StepVotes,
        ratification: ledger::StepVotes,
    ) -> Message {
        let hdr = node_data::message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            iteration,
            block_hash,
            topic: Topics::Quorum,
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());
        let payload = payload::Quorum {
            signature,
            validation,
            ratification,
        };

        Message::new_quorum(hdr, payload)
    }

    pub(crate) fn reset(&mut self, iteration: u8) {
        self.validation_result = Default::default();
        self.curr_iteration = iteration;
    }

    pub(crate) fn validation_result(&self) -> &ValidationResult {
        &self.validation_result
    }

    fn unwrap_msg(msg: &Message) -> Result<&Ratification, ConsensusError> {
        match &msg.payload {
            Payload::Ratification(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }

    /// Verifies either valid or nil quorum of validation output
    fn verify_validation_result(
        ru: &RoundUpdate,
        iter: u8,
        round_committees: &RoundCommittees,
        result: &ValidationResult,
    ) -> Result<(), ConsensusError> {
        match result.quorum {
            QuorumType::ValidQuorum | QuorumType::NilQuorum => {
                if let Some(generator) = round_committees.get_generator(iter) {
                    if let Some(validation_committee) =
                        round_committees.get_validation_committee(iter)
                    {
                        let cfg = sortition::Config::new(
                            ru.seed(),
                            ru.round,
                            StepName::Validation.to_step(iter),
                            config::VALIDATION_COMMITTEE_SIZE,
                            Some(generator),
                        );

                        verify_votes(
                            &result.hash,
                            result.sv.bitset,
                            &result.sv.aggregate_signature.inner(),
                            validation_committee,
                            &cfg,
                            true,
                        )?;

                        Ok(())
                    } else {
                        error!("could not get validation committee");
                        Err(ConsensusError::InvalidValidation)
                    }
                } else {
                    error!("could not get generator");
                    Err(ConsensusError::InvalidValidation)
                }
            }
            QuorumType::NoQuorum => Err(ConsensusError::InvalidValidation), /* TBD */
            QuorumType::InvalidQuorum => Err(ConsensusError::InvalidValidation), /* Not supported */
        }
    }
}
