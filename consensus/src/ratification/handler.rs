// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_trait::async_trait;
use node_data::StepName;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::Attestation;
use node_data::message::payload::{
    Quorum, Ratification, ValidationResult, Vote,
};
use node_data::message::{
    ConsensusHeader, Message, Payload, SignedStepMessage, StepMessage,
};

use tracing::{debug, error, info, warn};

use crate::aggregator::{Aggregator, StepVote};
use crate::commons::RoundUpdate;
use crate::config::is_emergency_iter;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::quorum::verifiers::verify_quorum_votes;
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;

pub struct RatificationHandler {
    pub(crate) att_registry: SafeAttestationInfoRegistry,

    pub(crate) aggregator: Aggregator<Ratification>,
    validation_result: ValidationResult,
    pub(crate) curr_iteration: u8,
}

// Implement the required trait to use Aggregator
impl StepVote for Ratification {
    fn vote(&self) -> &Vote {
        &self.vote
    }
}

impl RatificationHandler {
    pub fn verify_stateless(
        msg: &Message,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        match &msg.payload {
            Payload::Ratification(p) => {
                p.verify_signature()?;
                let signer = &p.sign_info.signer;
                let committee = round_committees
                    .get_committee(msg.get_step())
                    .expect("committee to be created before run");

                committee
                    .votes_for(signer)
                    .ok_or(ConsensusError::NotCommitteeMember)?;
            }

            Payload::ValidationQuorum(q) => Self::verify_validation_result(
                &q.header,
                &q.result,
                round_committees,
            )?,
            Payload::Empty => (),
            _ => {
                info!("cannot verify in validation handler");
                Err(ConsensusError::InvalidMsgType)?
            }
        }

        Ok(())
    }
}

#[async_trait]
impl MsgHandler for RatificationHandler {
    fn verify(
        &self,
        msg: &Message,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        if let Payload::Ratification(p) = &msg.payload {
            if self.aggregator.is_vote_collected(p) {
                return Err(ConsensusError::VoteAlreadyCollected);
            }

            p.verify_signature()?;

            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }

    /// Collect the Ratification message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
        round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        let vote = p.vote;
        let iteration = p.header().iteration;

        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            return Err(ConsensusError::InvalidMsgIteration(iteration));
        }

        // Ensure the vote matches the msg ValidationResult
        let vr_vote = *p.validation_result.vote();
        if vr_vote != vote {
            warn!(
                event = "Vote discarded",
                step = "Ratification",
                reason = "mismatch with msg ValidationResult",
                round = ru.round,
                iter = iteration
            );

            return Err(ConsensusError::VoteMismatch(vr_vote, vote));
        }

        if vote == Vote::NoQuorum {
            // If the vote is NoQuorum, ensure StepVotes is empty
            if !p.validation_result.step_votes().is_empty() {
                warn!(
                    event = "Vote discarded",
                    step = "Ratification",
                    reason = "mismatch with msg ValidationResult",
                    round = ru.round,
                    iter = iteration
                );
                return Err(ConsensusError::InvalidVote(vote));
            }
        } else {
            // If the vote is a Quorum, check it against our Validation result
            let local_vote = *self.validation_result().vote();
            match local_vote {
                Vote::NoQuorum => {
                    // If our result is NoQuorum, verify votes and
                    // then update our result
                    Self::verify_validation_result(
                        &p.header,
                        &p.validation_result,
                        round_committees,
                    )?;

                    self.update_validation_result(p.validation_result.clone())
                }

                _ => {
                    // If our result is also a Quorum, check they match and skip
                    // verification.
                    if vote != local_vote {
                        if !is_emergency_iter(iteration) {
                            warn!(
                                event = "Vote discarded",
                                step = "Ratification",
                                reason = "mismatch with local ValidationResult",
                                round = ru.round,
                                iter = iteration
                            );

                            return Err(ConsensusError::VoteMismatch(
                                local_vote, vote,
                            ));
                        } else {
                            // In Emergency Mode, we do not discard votes
                            // because multiple votes are allowed
                            Self::verify_validation_result(
                                &p.header,
                                &p.validation_result,
                                round_committees,
                            )?;

                            // We update our result to be sure to build a
                            // coherent Quorum message
                            self.update_validation_result(
                                p.validation_result.clone(),
                            )
                        }
                    }
                }
            }
        }

        // Collect vote
        let (step_votes, quorum_reached) = self
            .aggregator
            .collect_vote(committee, &p)
            .map_err(|error| {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    ?vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
                ConsensusError::InvalidVote(vote)
            })?;

        // Record updated Ratification StepVotes in global registry
        // If we reached a quorum on both steps, return the Quorum message
        if let Some(attestation) =
            self.att_registry.lock().await.set_step_votes(
                iteration,
                &vote,
                step_votes,
                StepName::Ratification,
                quorum_reached,
                &generator.expect("There must be a valid generator"),
            )
        {
            let ch = ConsensusHeader {
                prev_block_hash: ru.hash(),
                round: ru.round,
                iteration,
            };
            let qmsg = Self::build_quorum(ch, attestation);
            return Ok(StepOutcome::Ready(qmsg));
        }

        Ok(StepOutcome::Pending)
    }

    /// Collects the ratification message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // Collect vote, if msg payload is ratification type
        let collect_vote = self.aggregator.collect_vote(committee, &p);

        match collect_vote {
            Ok((step_votes, quorum_reached)) => {
                // Record any signature in global registry
                if let Some(attestation) =
                    self.att_registry.lock().await.set_step_votes(
                        p.header().iteration,
                        &p.vote,
                        step_votes,
                        StepName::Ratification,
                        quorum_reached,
                        &generator.expect("There must be a valid generator"),
                    )
                {
                    let qmsg = Self::build_quorum(p.header(), attestation);
                    return Ok(StepOutcome::Ready(qmsg));
                }
            }

            Err(error) => {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    vote = ?p.vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
            }
        };

        Ok(StepOutcome::Pending)
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(
        &self,
        _ru: &RoundUpdate,
        _curr_iteration: u8,
    ) -> Option<Message> {
        None
    }
}

impl RatificationHandler {
    pub(crate) fn new(att_registry: SafeAttestationInfoRegistry) -> Self {
        Self {
            att_registry,
            aggregator: Default::default(),
            validation_result: Default::default(),
            curr_iteration: 0,
        }
    }

    fn build_quorum(header: ConsensusHeader, att: Attestation) -> Message {
        let payload = Quorum { header, att };

        payload.into()
    }

    pub(crate) fn reset(&mut self, iter: u8, validation: ValidationResult) {
        self.validation_result = validation;
        self.curr_iteration = iter;
    }

    pub(crate) fn validation_result(&self) -> &ValidationResult {
        &self.validation_result
    }

    pub(crate) fn update_validation_result(&mut self, vr: ValidationResult) {
        let cur_vote = *self.validation_result().vote();
        let new_vote = vr.vote();
        debug!(
            "Update local ValidationResult ({:?}) with {:?}",
            cur_vote, new_vote
        );

        self.validation_result = vr;
    }

    fn unwrap_msg(msg: Message) -> Result<Ratification, ConsensusError> {
        match msg.payload {
            Payload::Ratification(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }

    /// Verifies either valid or nil quorum of validation output
    pub(crate) fn verify_validation_result(
        header: &ConsensusHeader,
        result: &ValidationResult,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let iter = header.iteration;
        let validation_committee = round_committees
            .get_validation_committee(iter)
            .ok_or_else(|| {
                error!("could not get validation committee");
                ConsensusError::CommitteeNotGenerated
            })?;
        verify_quorum_votes(
            header,
            StepName::Validation,
            result.vote(),
            result.step_votes(),
            validation_committee,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use dusk_core::signatures::bls::{
        PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    };
    use node_data::StepName;
    use node_data::ledger::{Header, Seed, StepVotes};
    use node_data::message::payload::{QuorumType, ValidationResult, Vote};
    use node_data::message::{Message, Payload};
    use rand::SeedableRng;
    use tokio::sync::Mutex;

    use crate::commons::RoundUpdate;
    use crate::errors::ConsensusError;
    use crate::iteration_ctx::RoundCommittees;
    use crate::msg_handler::MsgHandler;
    use crate::ratification::handler::RatificationHandler;
    use crate::step_votes_reg::{AttInfoRegistry, SafeAttestationInfoRegistry};
    use crate::user::committee::Committee;
    use crate::user::provisioners::{DUSK, Provisioners};
    use crate::user::sortition::Config;

    // Keep one deterministic seed per test setup to isolate key material.
    const SEED_ACCEPT_MATCHING_RESULT: u64 = 1;
    const SEED_ACCEPT_PAST_PENDING: u64 = 2;
    const SEED_ACCEPT_PAST_READY: u64 = 3;
    const SEED_REJECT_NOQUORUM: u64 = 4;
    const SEED_REJECT_MISMATCH: u64 = 5;
    const SEED_REJECT_PAST_INVALID_PAYLOAD: u64 = 6;

    // Build a single-member ratification committee and matching signer.
    fn setup_committee(
        seed: Seed,
        round: u64,
        iteration: u8,
        rng_seed: u64,
    ) -> (Committee, RoundUpdate) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(rng_seed);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));

        let mut provisioners = Provisioners::empty();
        provisioners.add_provisioner_with_value(pk.clone(), 1000 * DUSK);

        let mut tip_header = Header::default();
        tip_header.height = round - 1;
        tip_header.seed = seed;

        let ru = RoundUpdate::new(
            pk.clone(),
            sk,
            &tip_header,
            HashMap::new(),
            vec![],
        );
        let cfg =
            Config::new(seed, round, iteration, StepName::Ratification, vec![]);
        let committee = Committee::new(&provisioners, &cfg);

        (committee, ru)
    }

    // Build a ratification handler initialized for one iteration.
    fn build_handler(
        iteration: u8,
        validation_result: ValidationResult,
    ) -> RatificationHandler {
        let att_registry = Arc::new(Mutex::new(AttInfoRegistry::new()));
        let mut handler = RatificationHandler::new(att_registry);
        handler.reset(iteration, validation_result);
        handler
    }

    // Build a ratification handler and expose its shared attestation registry.
    fn build_handler_with_registry(
        iteration: u8,
        validation_result: ValidationResult,
    ) -> (RatificationHandler, SafeAttestationInfoRegistry) {
        let att_registry = Arc::new(Mutex::new(AttInfoRegistry::new()));
        let mut handler = RatificationHandler::new(att_registry.clone());
        handler.reset(iteration, validation_result);
        (handler, att_registry)
    }

    // Collect one ratification message using the default round committees.
    async fn collect_ratification_message(
        handler: &mut RatificationHandler,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<crate::msg_handler::StepOutcome, ConsensusError> {
        handler
            .collect(
                msg,
                ru,
                committee,
                Some(*ru.pubkey_bls.bytes()),
                &RoundCommittees::default(),
            )
            .await
    }

    // Collect one ratification message from a past iteration.
    async fn collect_ratification_message_from_past(
        handler: &mut RatificationHandler,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<crate::msg_handler::StepOutcome, ConsensusError> {
        handler
            .collect_from_past(msg, committee, Some(*ru.pubkey_bls.bytes()))
            .await
    }

    #[tokio::test]
    // Matching ratification and local validation results should be accepted.
    async fn ratification_accepts_matching_local_validation_result() {
        let (committee, ru) = setup_committee(
            Seed::from([8u8; 48]),
            1,
            0,
            SEED_ACCEPT_MATCHING_RESULT,
        );

        let local_validation = ValidationResult::new(
            StepVotes::new([1u8; 48], 1),
            Vote::NoCandidate,
            QuorumType::NoCandidate,
        );
        let mut handler = build_handler(0, local_validation.clone());

        let ratification =
            crate::build_ratification_payload(&ru, 0, &local_validation);
        let msg: Message = ratification.into();

        let outcome =
            collect_ratification_message(&mut handler, msg, &ru, &committee)
                .await;
        assert!(
            matches!(outcome, Ok(crate::msg_handler::StepOutcome::Pending)),
            "expected accepted vote to be collected"
        );
    }

    #[tokio::test]
    // Past ratification without validation quorum should remain pending.
    async fn ratification_collect_from_past_stays_pending_without_validation_quorum()
     {
        let (committee, ru) = setup_committee(
            Seed::from([10u8; 48]),
            1,
            0,
            SEED_ACCEPT_PAST_PENDING,
        );
        let local_validation = ValidationResult::new(
            StepVotes::new([1u8; 48], 1),
            Vote::NoCandidate,
            QuorumType::NoCandidate,
        );
        let mut handler = build_handler(0, local_validation.clone());

        let ratification =
            crate::build_ratification_payload(&ru, 0, &local_validation);
        let msg: Message = ratification.into();

        let outcome = collect_ratification_message_from_past(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await;

        assert!(
            matches!(outcome, Ok(crate::msg_handler::StepOutcome::Pending)),
            "expected pending while validation quorum is missing"
        );
    }

    #[tokio::test]
    // Past ratification should emit Quorum once validation and ratification quorums are ready.
    async fn ratification_collect_from_past_emits_quorum_when_ready() {
        let (committee, ru) = setup_committee(
            Seed::from([12u8; 48]),
            1,
            0,
            SEED_ACCEPT_PAST_READY,
        );
        let (mut handler, att_registry) =
            build_handler_with_registry(0, ValidationResult::default());

        let vote = Vote::Valid([7u8; 32]);
        let validation_votes = StepVotes::new([3u8; 48], 1);
        att_registry.lock().await.set_step_votes(
            0,
            &vote,
            validation_votes,
            StepName::Validation,
            true,
            ru.pubkey_bls.bytes(),
        );

        let msg_validation = ValidationResult::new(
            StepVotes::new([4u8; 48], 1),
            vote,
            QuorumType::Valid,
        );
        let ratification =
            crate::build_ratification_payload(&ru, 0, &msg_validation);
        let msg: Message = ratification.into();

        let outcome = collect_ratification_message_from_past(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        .expect("expected ready quorum output");

        match outcome {
            crate::msg_handler::StepOutcome::Ready(msg) => match msg.payload {
                Payload::Quorum(q) => {
                    assert_eq!(q.att.result.vote(), &vote);
                }
                _ => panic!("expected quorum payload"),
            },
            crate::msg_handler::StepOutcome::Pending => {
                panic!("expected quorum-ready output")
            }
        }
    }

    #[tokio::test]
    // NoQuorum ratification must not carry validation step votes.
    async fn ratification_rejects_noquorum_with_validation_votes() {
        let (committee, ru) =
            setup_committee(Seed::from([4u8; 48]), 1, 0, SEED_REJECT_NOQUORUM);
        let mut handler = build_handler(0, ValidationResult::default());

        let validation = ValidationResult::new(
            StepVotes::new([9u8; 48], 1),
            Vote::NoQuorum,
            QuorumType::NoQuorum,
        );
        let ratification =
            crate::build_ratification_payload(&ru, 0, &validation);
        let msg: Message = ratification.into();

        let err = match collect_ratification_message(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected invalid noquorum rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::InvalidVote(Vote::NoQuorum)));
    }

    #[tokio::test]
    // Ratification vote must match the node's stored local validation result.
    async fn ratification_rejects_mismatched_local_validation_result() {
        let (committee, ru) =
            setup_committee(Seed::from([6u8; 48]), 1, 0, SEED_REJECT_MISMATCH);

        let local_vote = Vote::Valid([2u8; 32]);
        let local_validation = ValidationResult::new(
            StepVotes::new([1u8; 48], 1),
            local_vote,
            QuorumType::Valid,
        );
        let mut handler = build_handler(0, local_validation);

        let msg_validation = ValidationResult::new(
            StepVotes::new([3u8; 48], 1),
            Vote::NoCandidate,
            QuorumType::NoCandidate,
        );
        let ratification =
            crate::build_ratification_payload(&ru, 0, &msg_validation);
        let msg: Message = ratification.into();

        let err = match collect_ratification_message(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected vote mismatch rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::VoteMismatch(_, _)));
    }

    #[tokio::test]
    // Past ratification handling must reject non-ratification payloads.
    async fn ratification_collect_from_past_rejects_invalid_payload() {
        let (committee, ru) = setup_committee(
            Seed::from([14u8; 48]),
            1,
            0,
            SEED_REJECT_PAST_INVALID_PAYLOAD,
        );
        let mut handler = build_handler(0, ValidationResult::default());

        let err = match collect_ratification_message_from_past(
            &mut handler,
            Message::default(),
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected invalid payload rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::InvalidMsgType));
    }
}
