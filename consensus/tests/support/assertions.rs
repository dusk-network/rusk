// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;

use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::{Message, Payload};
use node_data::StepName;

use dusk_consensus::quorum::verifiers::verify_quorum_votes;

use super::committee::build_committee_for_round;
use super::{Envelope, TestNetwork};

pub fn assert_quorum_message_invariants(msg: &Message) {
    let Payload::Quorum(q) = &msg.payload else {
        panic!("expected quorum payload");
    };

    assert_eq!(msg.header, q.header, "quorum header mismatch");

    let validation_sig_nonzero = q
        .att
        .validation
        .aggregate_signature()
        .inner()
        .iter()
        .any(|b| *b != 0);
    let ratification_sig_nonzero = q
        .att
        .ratification
        .aggregate_signature()
        .inner()
        .iter()
        .any(|b| *b != 0);

    assert_eq!(
        q.att.validation.bitset == 0,
        !validation_sig_nonzero,
        "validation StepVotes signature/bitset mismatch"
    );
    assert_eq!(
        q.att.ratification.bitset == 0,
        !ratification_sig_nonzero,
        "ratification StepVotes signature/bitset mismatch"
    );

    match q.att.result {
        RatificationResult::Success(Vote::Valid(_)) => {}
        RatificationResult::Success(other) => {
            panic!("unexpected success vote: {other:?}");
        }
        RatificationResult::Fail(Vote::Valid(_)) => {
            panic!("valid vote should not be a failure");
        }
        RatificationResult::Fail(_) => {}
    }

    if matches!(q.att.result.vote(), Vote::NoQuorum) {
        assert!(
            q.att.validation.is_empty(),
            "noquorum must have empty validation votes"
        );
    } else {
        assert!(
            !q.att.validation.is_empty(),
            "quorum requires validation votes"
        );
    }

    assert!(
        !q.att.ratification.is_empty(),
        "quorum requires ratification votes"
    );
}

pub fn assert_quorum_message_verifies(network: &TestNetwork, msg: &Message) {
    let Payload::Quorum(q) = &msg.payload else {
        panic!("expected quorum payload");
    };

    let seed = network.tip_header.seed;
    let round = q.header.round;
    let iter = q.header.iteration;
    let vote = q.att.result.vote();

    if !matches!(vote, Vote::NoQuorum) {
        let validation_committee = build_committee_for_round(
            network,
            seed,
            round,
            iter,
            StepName::Validation,
        );
        verify_quorum_votes(
            &q.header,
            StepName::Validation,
            vote,
            &q.att.validation,
            &validation_committee,
        )
        .expect("validation step votes verify");
    }

    let ratification_committee = build_committee_for_round(
        network,
        seed,
        round,
        iter,
        StepName::Ratification,
    );
    verify_quorum_votes(
        &q.header,
        StepName::Ratification,
        vote,
        &q.att.ratification,
        &ratification_committee,
    )
    .expect("ratification step votes verify");
}

pub fn assert_quorum_message_ok(network: &TestNetwork, msg: &Message) {
    assert_quorum_message_invariants(msg);
    assert_quorum_message_verifies(network, msg);
}

pub fn assert_no_conflicting_quorums(
    envelopes: &[Envelope],
    seen: &mut HashMap<(u64, u8), RatificationResult>,
) {
    for env in envelopes {
        if let Payload::Quorum(q) = &env.msg.payload {
            let key = (q.header.round, q.header.iteration);
            if let Some(prev) = seen.get(&key) {
                assert_eq!(
                    prev, &q.att.result,
                    "conflicting quorum for round/iter {:?}",
                    key
                );
            } else {
                seen.insert(key, q.att.result);
            }
        }
    }
}

pub fn assert_quorum_batch_invariants_with_network(
    envelopes: &[Envelope],
    seen: &mut HashMap<(u64, u8), RatificationResult>,
    network: &TestNetwork,
) {
    assert_quorum_batch_invariants_with_network_for_round(
        envelopes, seen, network, None,
    );
}

pub fn assert_quorum_batch_invariants_with_network_for_round(
    envelopes: &[Envelope],
    seen: &mut HashMap<(u64, u8), RatificationResult>,
    network: &TestNetwork,
    verify_round: Option<u64>,
) {
    for env in envelopes {
        if let Payload::Quorum(q) = &env.msg.payload {
            assert_quorum_message_invariants(&env.msg);
            if verify_round.map_or(true, |round| round == q.header.round) {
                assert_quorum_message_verifies(network, &env.msg);
            }
        }
    }
    assert_no_conflicting_quorums(envelopes, seen);
}
