// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use async_trait::async_trait;
use node_data::StepName;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Block, StepVotes, to_str};
use node_data::message::payload::{
    GetResource, Inv, QuorumType, Validation, Vote,
};
use node_data::message::{
    ConsensusHeader, Message, Payload, SignedStepMessage, StepMessage, payload,
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::aggregator::{Aggregator, StepVote};
use crate::commons::{Database, RoundUpdate};
use crate::config::is_emergency_iter;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;

pub struct ValidationHandler<D: Database> {
    pub(crate) aggr: Aggregator<Validation>,
    pub(crate) candidate: Option<Block>,
    att_registry: SafeAttestationInfoRegistry,
    curr_iteration: u8,
    pub(crate) db: Arc<Mutex<D>>,
}

// Implement the required trait to use Aggregator
impl StepVote for Validation {
    fn vote(&self) -> &Vote {
        &self.vote
    }
}

pub fn verify_stateless(
    msg: &Message,
    round_committees: &RoundCommittees,
) -> Result<(), ConsensusError> {
    match &msg.payload {
        Payload::Validation(p) => {
            p.verify_signature()?;

            let signer = &p.sign_info.signer;
            let committee = round_committees
                .get_committee(msg.get_step())
                .expect("committee to be created before run");

            committee
                .votes_for(signer)
                .ok_or(ConsensusError::NotCommitteeMember)?;
        }
        Payload::Empty => (),
        _ => {
            info!("cannot verify in validation handler");
            Err(ConsensusError::InvalidMsgType)?
        }
    }

    Ok(())
}

impl<D: Database> ValidationHandler<D> {
    pub(crate) fn new(
        att_registry: SafeAttestationInfoRegistry,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            att_registry,
            aggr: Aggregator::default(),
            candidate: None,
            curr_iteration: 0,
            db,
        }
    }

    pub(crate) fn reset(&mut self, curr_iteration: u8) {
        self.candidate = None;
        self.curr_iteration = curr_iteration;
    }

    fn unwrap_msg(msg: Message) -> Result<Validation, ConsensusError> {
        match msg.payload {
            Payload::Validation(r) => Ok(r),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }

    async fn build_validation_result(
        &self,
        step_votes: StepVotes,
        vote: Vote,
        quorum: QuorumType,
        consensus_header: &ConsensusHeader,
    ) -> Message {
        let vr = payload::ValidationResult::new(step_votes, vote, quorum);

        // In Emergency Mode, we store ValidationResult in case some peer
        // requests it
        if is_emergency_iter(consensus_header.iteration) {
            debug!(
              event = "Store ValidationResult",
              info = ?consensus_header,
              src = "Validation"
            );

            self.db
                .lock()
                .await
                .store_validation_result(consensus_header, &vr)
                .await;
        }

        Message::from(vr)
    }
}

#[async_trait]
impl<D: Database> MsgHandler for ValidationHandler<D> {
    /// Verifies if a msg is a valid validation message.
    fn verify(
        &self,
        msg: &Message,
        _round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        match &msg.payload {
            Payload::Validation(p) => {
                if self.aggr.is_vote_collected(p) {
                    return Err(ConsensusError::VoteAlreadyCollected);
                }

                p.verify_signature()?
            }
            Payload::Empty => (),
            _ => Err(ConsensusError::InvalidMsgType)?,
        };

        Ok(())
    }

    /// Collects the validation message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
        _round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        let iteration = p.header().iteration;
        if iteration != self.curr_iteration {
            // Message that belongs to step from the past must be handled with
            // collect_from_past fn
            return Err(ConsensusError::InvalidMsgIteration(iteration));
        }

        let (step_votes, quorum_reached) =
            self.aggr.collect_vote(committee, &p).map_err(|error| {
                warn!(
                    event = "Cannot collect vote",
                    ?error,
                    from = p.sign_info().signer.to_bs58(),
                    vote = ?p.vote,
                    msg_step = p.get_step(),
                    msg_iter = p.header().iteration,
                    msg_height = p.header().round,
                );
                ConsensusError::InvalidVote(p.vote)
            })?;
        // Record result in global round registry
        _ = self.att_registry.lock().await.set_step_votes(
            iteration,
            &p.vote,
            step_votes,
            StepName::Validation,
            quorum_reached,
            &generator.expect("There must be a valid generator"),
        );

        if quorum_reached {
            let vote = p.vote;

            let quorum_type = match vote {
                Vote::NoCandidate => QuorumType::NoCandidate,
                Vote::Invalid(_) => QuorumType::Invalid,
                Vote::Valid(_) => QuorumType::Valid,
                Vote::NoQuorum => {
                    return Err(ConsensusError::InvalidVote(vote));
                }
            };

            let vrmsg = self
                .build_validation_result(
                    step_votes,
                    vote,
                    quorum_type,
                    &p.header(),
                )
                .await;

            return Ok(StepOutcome::Ready(vrmsg));
        }

        Ok(StepOutcome::Pending)
    }

    /// Collects the validation message from former iteration.
    async fn collect_from_past(
        &mut self,
        msg: Message,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError> {
        if is_emergency_iter(msg.header.iteration) {
            if let Payload::ValidationQuorum(vq) = msg.payload {
                if !vq.result.vote().is_valid() {
                    return Err(ConsensusError::InvalidMsgType);
                };

                let vr = vq.result;

                // Store ValidationResult
                debug!(
                  event = "Store ValidationResult",
                  info = ?vq.header,
                  src = "ValidationQuorum"
                );

                self.db
                    .lock()
                    .await
                    .store_validation_result(&vq.header, &vr)
                    .await;

                // Extract the ValidationResult and return it as msg
                let vr_msg = vr.into();
                return Ok(StepOutcome::Ready(vr_msg));
            }
        }

        let p = Self::unwrap_msg(msg)?;

        // NoQuorum cannot be cast from validation committee
        if p.vote == Vote::NoQuorum {
            return Err(ConsensusError::InvalidVote(p.vote));
        }

        // Collect vote, if msg payload is validation type
        let collect_vote = self.aggr.collect_vote(committee, &p);

        match collect_vote {
            Ok((step_votes, validation_quorum_reached)) => {
                // We ignore the result since it's not possible to have a full
                // quorum in the validation step
                let _ = self.att_registry.lock().await.set_step_votes(
                    p.header().iteration,
                    &p.vote,
                    step_votes,
                    StepName::Validation,
                    validation_quorum_reached,
                    &generator.expect("There must be a valid generator"),
                );

                if p.vote.is_valid() && validation_quorum_reached {
                    // ValidationResult from past iteration is found
                    let vr = self
                        .build_validation_result(
                            step_votes,
                            p.vote,
                            QuorumType::Valid,
                            &p.header(),
                        )
                        .await;

                    return Ok(StepOutcome::Ready(vr));
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
        }
        Ok(StepOutcome::Pending)
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message> {
        if is_emergency_iter(curr_iteration) {
            // In Emergency Mode we request the ValidationResult from our peers
            // in case we arrived late and missed the votes

            let prev_block_hash = ru.hash();
            let round = ru.round;

            debug!(
                event = "Request ValidationResult",
                round,
                iteration = curr_iteration,
                prev_block = to_str(&prev_block_hash)
            );

            let mut inv = Inv::new(1);
            inv.add_validation_result(ConsensusHeader {
                prev_block_hash,
                round,
                iteration: curr_iteration,
            });
            let msg = GetResource::new(inv, None, u64::MAX, 0);
            return Some(msg.into());
        }

        None
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
    use node_data::message::payload::{
        QuorumType, ValidationQuorum, ValidationResult, Vote,
    };
    use node_data::message::{ConsensusHeader, Message, Payload};
    use rand::SeedableRng;
    use tokio::sync::Mutex;

    use crate::commons::{Database, RoundUpdate};
    use crate::config::EMERGENCY_MODE_ITERATION_THRESHOLD;
    use crate::errors::ConsensusError;
    use crate::iteration_ctx::RoundCommittees;
    use crate::msg_handler::MsgHandler;
    use crate::step_votes_reg::AttInfoRegistry;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{DUSK, Provisioners};
    use crate::user::sortition::Config;
    use crate::validation::handler::ValidationHandler;

    // Keep one deterministic seed per test setup to isolate key material.
    const SEED_ACCEPT_VALID_VOTE: u64 = 1;
    const SEED_ACCEPT_PAST_VALID_VOTE: u64 = 2;
    const SEED_REJECT_NOQUORUM: u64 = 3;
    const SEED_REJECT_WRONG_ITERATION: u64 = 4;
    const SEED_REJECT_PAST_NOQUORUM: u64 = 5;

    #[derive(Default)]
    struct DummyDb;

    // Minimal DB stub for validation handler behavior tests.
    #[async_trait::async_trait]
    impl Database for DummyDb {
        async fn store_candidate_block(
            &mut self,
            _b: node_data::ledger::Block,
        ) {
        }
        async fn store_validation_result(
            &mut self,
            _ch: &node_data::message::ConsensusHeader,
            _vr: &ValidationResult,
        ) {
        }
        async fn get_last_iter(&self) -> (node_data::ledger::Hash, u8) {
            ([0u8; 32], 0)
        }
        async fn store_last_iter(
            &mut self,
            _data: (node_data::ledger::Hash, u8),
        ) {
        }
    }

    // Build a single-member validation committee and matching signer.
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
            Config::new(seed, round, iteration, StepName::Validation, vec![]);
        let committee = Committee::new(&provisioners, &cfg);

        (committee, ru)
    }

    // Build a validation handler for a specific iteration.
    fn build_handler(iteration: u8) -> ValidationHandler<DummyDb> {
        let att_registry = Arc::new(Mutex::new(AttInfoRegistry::new()));
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let mut handler = ValidationHandler::new(att_registry, db);
        handler.reset(iteration);
        handler
    }

    // Collect a validation message for the current iteration.
    async fn collect_validation_message(
        handler: &mut ValidationHandler<DummyDb>,
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
                // RoundCommittees is not used in validation handler, so we can pass an empty one.
                &RoundCommittees::default(),
            )
            .await
    }

    // Collect a validation message from a past iteration.
    async fn collect_validation_message_from_past(
        handler: &mut ValidationHandler<DummyDb>,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
    ) -> Result<crate::msg_handler::StepOutcome, ConsensusError> {
        handler
            .collect_from_past(msg, committee, Some(*ru.pubkey_bls.bytes()))
            .await
    }

    // Build an emergency-mode ValidationQuorum message for past handling.
    fn build_validation_quorum_message(
        vote: Vote,
        quorum: QuorumType,
    ) -> Message {
        let header = ConsensusHeader {
            prev_block_hash: [7u8; 32],
            round: 1,
            iteration: EMERGENCY_MODE_ITERATION_THRESHOLD,
        };
        let result =
            ValidationResult::new(StepVotes::new([9u8; 48], 1), vote, quorum);
        let quorum = ValidationQuorum { header, result };
        quorum.into()
    }

    #[tokio::test]
    // Basic in-committee vote should be accepted when iteration matches.
    async fn validation_accepts_valid_vote() {
        let (committee, ru) = setup_committee(
            Seed::from([9u8; 48]),
            1,
            0,
            SEED_ACCEPT_VALID_VOTE,
        );
        let mut handler = build_handler(0);

        let validation =
            crate::build_validation_payload(Vote::NoCandidate, &ru, 0);
        let msg: Message = validation.into();

        let outcome =
            collect_validation_message(&mut handler, msg, &ru, &committee)
                .await;

        if let Err(err) = outcome {
            panic!("expected Ok outcome, got {err:?}");
        }
    }

    #[tokio::test]
    // Past valid votes should yield a ValidationResult when quorum is reached.
    async fn validation_collect_from_past_accepts_valid_vote_quorum() {
        let (committee, ru) = setup_committee(
            Seed::from([11u8; 48]),
            1,
            0,
            SEED_ACCEPT_PAST_VALID_VOTE,
        );
        let mut handler = build_handler(0);

        let vote = Vote::Valid([8u8; 32]);
        let validation = crate::build_validation_payload(vote, &ru, 0);
        let msg: Message = validation.into();

        let outcome = collect_validation_message_from_past(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        .expect("past valid vote should be accepted");

        match outcome {
            crate::msg_handler::StepOutcome::Ready(msg) => {
                if let Payload::ValidationResult(vr) = msg.payload {
                    assert_eq!(vr.vote(), &vote);
                } else {
                    panic!("expected ValidationResult output");
                }
            }
            crate::msg_handler::StepOutcome::Pending => {
                panic!("expected quorum-ready output")
            }
        }
    }

    #[tokio::test]
    // Emergency-mode ValidationQuorum with Valid vote should be accepted.
    async fn validation_collect_from_past_accepts_emergency_validation_quorum()
    {
        let mut handler = build_handler(0);
        let committee = Committee::default();
        let vote = Vote::Valid([5u8; 32]);
        let msg = build_validation_quorum_message(vote, QuorumType::Valid);

        let outcome = handler
            .collect_from_past(msg, &committee, None)
            .await
            .expect("valid emergency quorum should be accepted");

        match outcome {
            crate::msg_handler::StepOutcome::Ready(msg) => {
                if let Payload::ValidationResult(vr) = msg.payload {
                    assert_eq!(vr.vote(), &vote);
                } else {
                    panic!("expected ValidationResult output");
                }
            }
            crate::msg_handler::StepOutcome::Pending => {
                panic!("expected ready output from emergency quorum")
            }
        }
    }

    #[tokio::test]
    // Validation messages with NoQuorum vote should be rejected since it's not a valid vote.
    async fn validation_rejects_noquorum_vote() {
        let (committee, ru) =
            setup_committee(Seed::from([7u8; 48]), 1, 0, SEED_REJECT_NOQUORUM);
        let mut handler = build_handler(0);

        let validation =
            crate::build_validation_payload(Vote::NoQuorum, &ru, 0);
        let msg: Message = validation.into();

        let err = match collect_validation_message(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected noquorum vote rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::InvalidVote(Vote::NoQuorum)));
    }

    #[tokio::test]
    // Validation messages from future iterations should be rejected.
    async fn validation_rejects_wrong_iteration() {
        let (committee, ru) = setup_committee(
            Seed::from([5u8; 48]),
            1,
            0,
            SEED_REJECT_WRONG_ITERATION,
        );
        let mut handler = build_handler(0);

        let validation =
            crate::build_validation_payload(Vote::NoCandidate, &ru, 1);
        let msg: Message = validation.into();

        let err = match collect_validation_message(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected future iteration rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::InvalidMsgIteration(1)));
    }

    #[tokio::test]
    // Emergency-mode ValidationQuorum must carry a Valid vote.
    async fn validation_collect_from_past_rejects_emergency_non_valid_quorum() {
        let mut handler = build_handler(0);
        let committee = Committee::default();
        let msg = build_validation_quorum_message(
            Vote::NoCandidate,
            QuorumType::NoCandidate,
        );

        let err = match handler.collect_from_past(msg, &committee, None).await {
            Ok(_) => panic!("expected non-valid emergency quorum rejection"),
            Err(err) => err,
        };
        assert!(matches!(err, ConsensusError::InvalidMsgType));
    }

    #[tokio::test]
    // Past NoQuorum votes must be rejected in validation handling.
    async fn validation_collect_from_past_rejects_noquorum_vote() {
        let (committee, ru) = setup_committee(
            Seed::from([13u8; 48]),
            1,
            0,
            SEED_REJECT_PAST_NOQUORUM,
        );
        let mut handler = build_handler(0);

        let validation =
            crate::build_validation_payload(Vote::NoQuorum, &ru, 0);
        let msg: Message = validation.into();

        let err = match collect_validation_message_from_past(
            &mut handler,
            msg,
            &ru,
            &committee,
        )
        .await
        {
            Ok(_) => panic!("expected past noquorum vote rejection"),
            Err(err) => err,
        };

        assert!(matches!(err, ConsensusError::InvalidVote(Vote::NoQuorum)));
    }
}
