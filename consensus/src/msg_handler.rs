// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::message::{Message, Payload, Status};
use node_data::StepName;
use tracing::{debug, warn};

use crate::commons::RoundUpdate;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::ratification::handler::RatificationHandler;
use crate::user::committee::Committee;
use crate::{proposal, validation};

/// Indicates whether an output value is available for current step execution
/// (Step is Ready) or needs to collect data (Step is Pending)
#[allow(clippy::large_enum_variant)]
pub enum StepOutcome {
    Pending,
    Ready(Message),
}

/// MsgHandler must be implemented by any step that needs to handle an external
/// message within event_loop life-cycle.
#[async_trait]
pub trait MsgHandler {
    /// is_valid checks a new message is valid in the first place.
    ///
    /// Only if the message has correct round and step and is signed by a
    /// committee member then we delegate it to Phase::verify.
    fn is_valid(
        &self,
        msg: &Message,
        ru: &RoundUpdate,
        current_iteration: u8,
        step: StepName,
        committee: &Committee,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let signer = msg.get_signer();

        debug!(
            event = "validating msg",
            signer = signer.as_ref().map(|s| s.to_bs58()),
            src_addr = ?msg.metadata.as_ref().map(|m| m.src_addr),
            topic = ?msg.topic(),
            step = msg.get_step(),
            ray_id = msg.ray_id(),
        );

        // We don't verify the tip here, otherwise future round messages will be
        // discarded and not put into the queue
        let msg_tip = msg.header.prev_block_hash;
        match msg.compare(ru.round, current_iteration, step) {
            Status::Past => {
                Self::verify_message(msg, ru, round_committees, Status::Past)?;
                Err(ConsensusError::PastEvent)
            }
            Status::Present => {
                if msg_tip != ru.hash() {
                    return Err(ConsensusError::InvalidPrevBlockHash(msg_tip));
                }

                let signer = signer.ok_or(ConsensusError::InvalidMsgType)?;
                // Ensure the message originates from a committee member.
                if !committee.is_member(&signer) {
                    return Err(ConsensusError::NotCommitteeMember);
                }

                // Delegate message final verification to the phase instance.
                // It is the phase that knows what message type to expect and if
                // it is valid or not.
                self.verify(msg, round_committees)
            }
            Status::Future => {
                Self::verify_message(
                    msg,
                    ru,
                    round_committees,
                    Status::Future,
                )?;
                Err(ConsensusError::FutureEvent)
            }
        }
    }

    /// Verify step message for the current round with different iteration
    fn verify_message(
        msg: &Message,
        ru: &RoundUpdate,
        round_committees: &RoundCommittees,
        status: Status,
    ) -> Result<(), ConsensusError> {
        // Pre-verify messages for the current round with different iteration
        if msg.header.round == ru.round {
            let msg_tip = msg.header.prev_block_hash;
            if msg_tip != ru.hash() {
                return Err(ConsensusError::InvalidPrevBlockHash(msg_tip));
            }

            let step = msg.get_step();
            if let Some(committee) = round_committees.get_committee(step) {
                // Ensure msg is signed by a committee member.
                // We skip ValidationQuorum, since it has no signer
                if !matches!(msg.payload, Payload::ValidationQuorum(_)) {
                    let signer = msg.get_signer().expect("signer to exist");

                    if !committee.is_member(&signer) {
                        return Err(ConsensusError::NotCommitteeMember);
                    }
                }

                match &msg.payload {
                    node_data::message::Payload::Ratification(_)
                    | node_data::message::Payload::ValidationQuorum(_) => {
                        RatificationHandler::verify_stateless(
                            msg,
                            round_committees,
                        )?;
                    }
                    node_data::message::Payload::Validation(_) => {
                        validation::handler::verify_stateless(
                            msg,
                            round_committees,
                        )?;
                    }
                    node_data::message::Payload::Candidate(c) => {
                        proposal::handler::verify_stateless(
                            c,
                            round_committees,
                        )?;
                    }
                    _ => {
                        warn!(
                            "{status:?} message not repropagated {:?}",
                            msg.topic()
                        );
                        Err(ConsensusError::InvalidMsgType)?;
                    }
                }
            } else {
                warn!("{status:?} committee for step {step} not generated; skipping pre-verification for {:?} message", msg.topic());
            }
        }
        Ok(())
    }

    /// verify allows each Phase to fully verify the message payload.
    fn verify(
        &self,
        msg: &Message,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError>;

    /// collect allows each Phase to process a verified inbound message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
        round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError>;

    /// collect allows each Phase to process a verified message from a former
    /// iteration
    async fn collect_from_past(
        &mut self,
        msg: Message,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError>;

    /// handle_timeout allows each Phase to handle a timeout event.
    /// Returned Message here is sent to outboud queue.
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use dusk_core::signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};
    use node_data::ledger::{Block, Header, Seed, StepVotes};
    use node_data::message::payload::{Candidate, QuorumType, ValidationResult, Vote};
    use node_data::message::{Message, SignedStepMessage};
    use node_data::StepName;
    use tokio::sync::Mutex;
    use rand::SeedableRng;

    use crate::commons::{Database, RoundUpdate};
    use crate::errors::ConsensusError;
    use crate::iteration_ctx::RoundCommittees;
    use crate::msg_handler::MsgHandler;
    use crate::merkle::merkle_root;
    use crate::proposal::handler::ProposalHandler;
    use crate::ratification::handler::RatificationHandler;
    use crate::step_votes_reg::AttInfoRegistry;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, DUSK};
    use crate::user::sortition::Config;
    use crate::validation::handler::ValidationHandler;

    #[derive(Clone, Copy)]
    enum ExpectedOutcome {
        Ok,
        InvalidPrevHash,
        InvalidSignature,
        NotCommitteeMember,
        FutureEvent,
    }

    struct MutationCase {
        name: &'static str,
        msg: Message,
        expected: ExpectedOutcome,
    }

    fn assert_expected(
        result: Result<(), ConsensusError>,
        expected: ExpectedOutcome,
        name: &str,
    ) {
        match expected {
            ExpectedOutcome::Ok => {
                assert!(result.is_ok(), "{name}: expected Ok, got {result:?}");
            }
            ExpectedOutcome::InvalidPrevHash => {
                assert!(
                    matches!(
                        result,
                        Err(ConsensusError::InvalidPrevBlockHash(_))
                    ),
                    "{name}: expected InvalidPrevBlockHash, got {result:?}"
                );
            }
            ExpectedOutcome::InvalidSignature => {
                assert!(
                    matches!(result, Err(ConsensusError::InvalidSignature(_))),
                    "{name}: expected InvalidSignature, got {result:?}"
                );
            }
            ExpectedOutcome::NotCommitteeMember => {
                assert!(
                    matches!(result, Err(ConsensusError::NotCommitteeMember)),
                    "{name}: expected NotCommitteeMember, got {result:?}"
                );
            }
            ExpectedOutcome::FutureEvent => {
                assert!(
                    matches!(result, Err(ConsensusError::FutureEvent)),
                    "{name}: expected FutureEvent, got {result:?}"
                );
            }
        }
    }

    fn run_mutation_matrix<F>(cases: Vec<MutationCase>, validate: F)
    where
        F: Fn(&Message) -> Result<(), ConsensusError>,
    {
        for case in cases {
            let result = validate(&case.msg);
            assert_expected(result, case.expected, case.name);
        }
    }

    #[derive(Default)]
    struct DummyDb;

    #[async_trait::async_trait]
    impl Database for DummyDb {
        async fn store_candidate_block(&mut self, _b: Block) {}
        async fn store_validation_result(
            &mut self,
            _ch: &node_data::message::ConsensusHeader,
            _vr: &ValidationResult,
        ) {
        }
        async fn get_last_iter(&self) -> (node_data::ledger::Hash, u8) {
            ([0u8; 32], 0)
        }
        async fn store_last_iter(&mut self, _data: (node_data::ledger::Hash, u8)) {
        }
    }

    #[derive(Clone)]
    struct TestKey {
        sk: BlsSecretKey,
        pk: node_data::bls::PublicKey,
    }

    fn key(seed: u64) -> TestKey {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
        TestKey { sk, pk }
    }

    fn tip_header() -> Header {
        let mut header = Header::default();
        header.height = 0;
        header.timestamp = 1;
        header.seed = Seed::from([7u8; 48]);
        header.hash = [1u8; 32];
        header.state_hash = [2u8; 32];
        header
    }

    fn build_round_committees(
        provisioners: &Provisioners,
        seed: Seed,
        round: u64,
        iter: u8,
    ) -> RoundCommittees {
        let mut rc = RoundCommittees::default();
        for step in [
            StepName::Proposal,
            StepName::Validation,
            StepName::Ratification,
        ] {
            let cfg = Config::new(seed, round, iter, step, vec![]);
            let committee = Committee::new(provisioners, &cfg);
            rc.insert(step.to_step(iter), committee);
        }
        rc
    }

    fn build_candidate(
        ru: &RoundUpdate,
        generator: &TestKey,
        prev_hash: [u8; 32],
        iter: u8,
    ) -> Message {
        let mut header = Header::default();
        header.height = ru.round;
        header.iteration = iter;
        header.prev_block_hash = prev_hash;
        header.generator_bls_pubkey = *generator.pk.bytes();
        header.txroot = merkle_root::<[u8; 32]>(&[]);
        header.faultroot = merkle_root::<[u8; 32]>(&[]);

        let block = Block::new(header, vec![], vec![]).expect("valid block");
        let mut candidate = Candidate { candidate: block };
        candidate.sign(&generator.sk, generator.pk.inner());
        candidate.into()
    }

    fn corrupt_signature(mut msg: Message) -> Message {
        match &mut msg.payload {
            node_data::message::Payload::Candidate(c) => {
                let mut sig = *c.candidate.header().signature.inner();
                sig[0] ^= 0x01;
                c.candidate.set_signature(sig.into());
            }
            node_data::message::Payload::Validation(v) => {
                let mut sig = *v.sign_info.signature.inner();
                sig[0] ^= 0x01;
                v.sign_info.signature = sig.into();
            }
            node_data::message::Payload::Ratification(r) => {
                let mut sig = *r.sign_info.signature.inner();
                sig[0] ^= 0x01;
                r.sign_info.signature = sig.into();
            }
            _ => {}
        }
        msg
    }

    #[test]
    fn proposal_mutations_rejected() {
        let key_a = key(1);
        let key_b = key(2);

        let mut provisioners = Provisioners::empty();
        provisioners.add_provisioner_with_value(key_a.pk.clone(), 1000 * DUSK);

        let tip = tip_header();
        let ru = RoundUpdate::new(
            key_a.pk.clone(),
            key_a.sk.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let rc = build_round_committees(&provisioners, tip.seed, ru.round, 0);
        let proposal_step = StepName::Proposal.to_step(0);
        let proposal_committee = rc.get_committee(proposal_step).expect("committee");

        let db = Arc::new(Mutex::new(DummyDb::default()));
        let handler = ProposalHandler::new(db);

        let generator = proposal_committee.iter().next().expect("generator");
        let generator_key = if generator.bytes() == key_a.pk.bytes() {
            &key_a
        } else {
            &key_b
        };
        let valid = build_candidate(&ru, generator_key, ru.hash(), 0);
        let wrong_prev = build_candidate(&ru, generator_key, [9u8; 32], 0);
        let wrong_signer = build_candidate(&ru, &key_b, ru.hash(), 0);
        let future_iter = build_candidate(&ru, generator_key, ru.hash(), 1);
        let bad_sig = corrupt_signature(valid.clone());

        run_mutation_matrix(
            vec![
                MutationCase {
                    name: "valid",
                    msg: valid,
                    expected: ExpectedOutcome::Ok,
                },
                MutationCase {
                    name: "wrong prev hash",
                    msg: wrong_prev,
                    expected: ExpectedOutcome::InvalidPrevHash,
                },
                MutationCase {
                    name: "invalid signature",
                    msg: bad_sig,
                    expected: ExpectedOutcome::InvalidSignature,
                },
                MutationCase {
                    name: "wrong signer",
                    msg: wrong_signer,
                    expected: ExpectedOutcome::NotCommitteeMember,
                },
                MutationCase {
                    name: "future iteration",
                    msg: future_iter,
                    expected: ExpectedOutcome::FutureEvent,
                },
            ],
            |msg| {
                handler.is_valid(
                    msg,
                    &ru,
                    0,
                    StepName::Proposal,
                    proposal_committee,
                    &rc,
                )
            },
        );
    }

    #[test]
    fn validation_mutations_rejected_or_queued() {
        let key_a = key(3);
        let key_b = key(4);

        let mut provisioners = Provisioners::empty();
        provisioners.add_provisioner_with_value(key_a.pk.clone(), 1000 * DUSK);

        let tip = tip_header();
        let ru = RoundUpdate::new(
            key_a.pk.clone(),
            key_a.sk.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );
        let ru_bad = RoundUpdate::new(
            key_b.pk.clone(),
            key_b.sk.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let rc = build_round_committees(&provisioners, tip.seed, ru.round, 0);
        let validation_step = StepName::Validation.to_step(0);
        let validation_committee = rc.get_committee(validation_step).expect("committee");

        let att_registry = Arc::new(Mutex::new(AttInfoRegistry::new()));
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let handler = ValidationHandler::new(att_registry, db);

        let valid = crate::build_validation_payload(Vote::NoCandidate, &ru, 0);
        let valid_msg: Message = valid.into();

        let mut wrong_prev = valid_msg.clone();
        wrong_prev.header.prev_block_hash = [9u8; 32];
        if let node_data::message::Payload::Validation(v) = &mut wrong_prev.payload
        {
            v.header.prev_block_hash = [9u8; 32];
        }

        let wrong_signer =
            crate::build_validation_payload(Vote::NoCandidate, &ru_bad, 0);
        let wrong_msg: Message = wrong_signer.into();

        let future = crate::build_validation_payload(Vote::NoCandidate, &ru, 1);
        let future_msg: Message = future.into();
        let bad_sig = corrupt_signature(valid_msg.clone());

        run_mutation_matrix(
            vec![
                MutationCase {
                    name: "valid",
                    msg: valid_msg,
                    expected: ExpectedOutcome::Ok,
                },
                MutationCase {
                    name: "wrong prev hash",
                    msg: wrong_prev,
                    expected: ExpectedOutcome::InvalidPrevHash,
                },
                MutationCase {
                    name: "invalid signature",
                    msg: bad_sig,
                    expected: ExpectedOutcome::InvalidSignature,
                },
                MutationCase {
                    name: "wrong signer",
                    msg: wrong_msg,
                    expected: ExpectedOutcome::NotCommitteeMember,
                },
                MutationCase {
                    name: "future iteration",
                    msg: future_msg,
                    expected: ExpectedOutcome::FutureEvent,
                },
            ],
            |msg| {
                handler.is_valid(
                    msg,
                    &ru,
                    0,
                    StepName::Validation,
                    validation_committee,
                    &rc,
                )
            },
        );
    }

    #[test]
    fn ratification_mutations_rejected_or_queued() {
        let key_a = key(5);
        let key_b = key(6);

        let mut provisioners = Provisioners::empty();
        provisioners.add_provisioner_with_value(key_a.pk.clone(), 1000 * DUSK);

        let tip = tip_header();
        let ru = RoundUpdate::new(
            key_a.pk.clone(),
            key_a.sk.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );
        let ru_bad = RoundUpdate::new(
            key_b.pk.clone(),
            key_b.sk.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let rc = build_round_committees(&provisioners, tip.seed, ru.round, 0);
        let ratification_step = StepName::Ratification.to_step(0);
        let ratification_committee =
            rc.get_committee(ratification_step).expect("committee");

        let att_registry = Arc::new(Mutex::new(AttInfoRegistry::new()));
        let handler = RatificationHandler::new(att_registry);

        let validation_result = ValidationResult::new(
            StepVotes::default(),
            Vote::NoQuorum,
            QuorumType::NoQuorum,
        );
        let ratification =
            crate::build_ratification_payload(&ru, 0, &validation_result);
        let ratification_msg: Message = ratification.into();

        let mut wrong_prev = ratification_msg.clone();
        wrong_prev.header.prev_block_hash = [9u8; 32];
        if let node_data::message::Payload::Ratification(r) =
            &mut wrong_prev.payload
        {
            r.header.prev_block_hash = [9u8; 32];
        }

        let wrong_signer =
            crate::build_ratification_payload(&ru_bad, 0, &validation_result);
        let wrong_msg: Message = wrong_signer.into();

        let future =
            crate::build_ratification_payload(&ru, 1, &validation_result);
        let future_msg: Message = future.into();
        let bad_sig = corrupt_signature(ratification_msg.clone());

        run_mutation_matrix(
            vec![
                MutationCase {
                    name: "valid",
                    msg: ratification_msg,
                    expected: ExpectedOutcome::Ok,
                },
                MutationCase {
                    name: "wrong prev hash",
                    msg: wrong_prev,
                    expected: ExpectedOutcome::InvalidPrevHash,
                },
                MutationCase {
                    name: "invalid signature",
                    msg: bad_sig,
                    expected: ExpectedOutcome::InvalidSignature,
                },
                MutationCase {
                    name: "wrong signer",
                    msg: wrong_msg,
                    expected: ExpectedOutcome::NotCommitteeMember,
                },
                MutationCase {
                    name: "future iteration",
                    msg: future_msg,
                    expected: ExpectedOutcome::FutureEvent,
                },
            ],
            |msg| {
                handler.is_valid(
                    msg,
                    &ru,
                    0,
                    StepName::Ratification,
                    ratification_committee,
                    &rc,
                )
            },
        );
    }
}
