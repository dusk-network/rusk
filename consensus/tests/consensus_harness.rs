// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod support;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use node_data::ledger::{Block, Header};
use node_data::message::payload::{InvType, RatificationResult, Vote};
use node_data::message::{Message, Payload, SignedStepMessage};
use node_data::StepName;
use rand::rngs::StdRng;
use rand::{seq::SliceRandom, Rng, SeedableRng};

use dusk_consensus::build_validation_payload;
use dusk_consensus::config::{EMERGENCY_MODE_ITERATION_THRESHOLD, MAX_ROUND_DISTANCE};
use dusk_consensus::commons::{RoundUpdate, TimeoutSet};
use dusk_consensus::errors::ConsensusError;
use dusk_consensus::merkle::merkle_root;
use dusk_consensus::quorum::verifiers::verify_quorum_votes;
use dusk_consensus::user::committee::Committee;
use dusk_consensus::user::sortition::Config as SortitionConfig;

use support::{
    decode_message, deliver_all, encode_message, find_quorum,
    read_trace_with_meta, wait_for_quorum, write_trace_with_meta, BufferedRouter,
    TestNetwork, TraceEntry, TraceMeta,
};

type ConsensusCancel = tokio::sync::oneshot::Sender<i32>;
type ConsensusHandle = tokio::task::JoinHandle<Result<(), ConsensusError>>;

fn spawn_all(
    network: &TestNetwork,
    timeouts: TimeoutSet,
) -> (Vec<ConsensusCancel>, Vec<ConsensusHandle>) {
    let mut cancels = Vec::new();
    let mut handles = Vec::new();

    for node in &network.nodes {
        let ru = node.round_update(&network.tip_header, timeouts.clone());
        let (cancel, handle) =
            node.spawn_consensus(ru, network.provisioners.clone());
        cancels.push(cancel);
        handles.push(handle);
    }

    (cancels, handles)
}

async fn shutdown_all(cancels: Vec<ConsensusCancel>, handles: Vec<ConsensusHandle>) {
    for cancel in cancels {
        let _ = cancel.send(0);
    }
    for handle in handles {
        let _ = handle.await;
    }
}

async fn set_emergency_iteration(network: &TestNetwork) {
    let last_iter = (
        network.tip_header.hash,
        EMERGENCY_MODE_ITERATION_THRESHOLD,
    );
    for node in &network.nodes {
        let mut db = node.db.lock().await;
        db.last_iter = last_iter;
    }
}

const TRACE_NETWORK_SIZE: usize = 3;
const TRACE_NETWORK_SEED: u64 = 900;
const CANONICAL_TRACE_FILE: &str = "consensus-trace.log";
const MULTI_ROUND_TRACE_FILE: &str = "consensus-trace-multi-round.log";

fn canonical_trace_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(CANONICAL_TRACE_FILE)
}

fn multi_round_trace_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(MULTI_ROUND_TRACE_FILE)
}

fn build_committee(
    network: &TestNetwork,
    iteration: u8,
    step: StepName,
) -> Committee {
    let round = network.tip_header.height + 1;
    let mut exclusion = Vec::new();
    if step != StepName::Proposal {
        let cur_generator = network
            .provisioners
            .get_generator(iteration, network.tip_header.seed, round);
        exclusion.push(cur_generator);
        if dusk_consensus::config::exclude_next_generator(iteration) {
            let next_generator = network
                .provisioners
                .get_generator(iteration + 1, network.tip_header.seed, round);
            exclusion.push(next_generator);
        }
    }
    let cfg = SortitionConfig::new(
        network.tip_header.seed,
        round,
        iteration,
        step,
        exclusion,
    );
    Committee::new(&network.provisioners, &cfg)
}

fn corrupt_message_signature(msg: &node_data::message::Message) -> Option<node_data::message::Message> {
    let mut corrupted = msg.clone();
    match &mut corrupted.payload {
        Payload::Candidate(c) => {
            let mut sig = *c.candidate.header().signature.inner();
            sig[0] ^= 0x01;
            c.candidate.set_signature(sig.into());
            Some(corrupted)
        }
        Payload::Validation(v) => {
            let mut sig = *v.sign_info.signature.inner();
            sig[0] ^= 0x01;
            v.sign_info.signature = sig.into();
            Some(corrupted)
        }
        Payload::Ratification(r) => {
            let mut sig = *r.sign_info.signature.inner();
            sig[0] ^= 0x01;
            r.sign_info.signature = sig.into();
            Some(corrupted)
        }
        _ => None,
    }
}

fn build_candidate_message(ru: &RoundUpdate, iteration: u8) -> Message {
    let mut header = Header::default();
    header.height = ru.round;
    header.iteration = iteration;
    header.prev_block_hash = ru.hash();
    header.generator_bls_pubkey = *ru.pubkey_bls.bytes();
    header.txroot = merkle_root::<[u8; 32]>(&[]);
    header.faultroot = merkle_root::<[u8; 32]>(&[]);

    let block = Block::new(header, vec![], vec![]).expect("valid block");
    let mut candidate = node_data::message::payload::Candidate { candidate: block };
    candidate.sign(&ru.secret_key, ru.pubkey_bls.inner());
    candidate.into()
}

fn assert_quorum_message_invariants(msg: &Message) {
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

fn build_committee_for_round(
    network: &TestNetwork,
    seed: node_data::ledger::Seed,
    round: u64,
    iteration: u8,
    step: StepName,
) -> Committee {
    let mut exclusion = Vec::new();
    if step != StepName::Proposal {
        let cur_generator =
            network
                .provisioners
                .get_generator(iteration, seed, round);
        exclusion.push(cur_generator);
        if dusk_consensus::config::exclude_next_generator(iteration) {
            let next_generator = network
                .provisioners
                .get_generator(iteration + 1, seed, round);
            exclusion.push(next_generator);
        }
    }
    let cfg = SortitionConfig::new(seed, round, iteration, step, exclusion);
    Committee::new(&network.provisioners, &cfg)
}

fn assert_quorum_message_verifies(network: &TestNetwork, msg: &Message) {
    let Payload::Quorum(q) = &msg.payload else {
        panic!("expected quorum payload");
    };

    let seed = network.tip_header.seed;
    let round = q.header.round;
    let iter = q.header.iteration;
    let vote = q.att.result.vote();

    if !matches!(vote, Vote::NoQuorum) {
        let validation_committee =
            build_committee_for_round(network, seed, round, iter, StepName::Validation);
        verify_quorum_votes(
            &q.header,
            StepName::Validation,
            vote,
            &q.att.validation,
            &validation_committee,
        )
        .expect("validation step votes verify");
    }

    let ratification_committee =
        build_committee_for_round(network, seed, round, iter, StepName::Ratification);
    verify_quorum_votes(
        &q.header,
        StepName::Ratification,
        vote,
        &q.att.ratification,
        &ratification_committee,
    )
    .expect("ratification step votes verify");
}

fn assert_no_conflicting_quorums(
    envelopes: &[support::Envelope],
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

fn assert_quorum_batch_invariants_with_network(
    envelopes: &[support::Envelope],
    seen: &mut HashMap<(u64, u8), RatificationResult>,
    network: &TestNetwork,
) {
    assert_quorum_batch_invariants_with_network_for_round(
        envelopes,
        seen,
        network,
        None,
    );
}

fn assert_quorum_batch_invariants_with_network_for_round(
    envelopes: &[support::Envelope],
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

fn track_trace_round_prev(
    msg: &Message,
    expected_round: u64,
    expected_prev: [u8; 32],
    round_prev: &mut HashMap<u64, [u8; 32]>,
) {
    match msg.payload {
        Payload::Candidate(_)
        | Payload::Validation(_)
        | Payload::Ratification(_)
        | Payload::ValidationQuorum(_)
        | Payload::Quorum(_) => {
            let round = msg.header.round;
            let prev = msg.header.prev_block_hash;
            if let Some(existing) = round_prev.get(&round) {
                assert_eq!(
                    existing, &prev,
                    "inconsistent prev_block_hash in trace for round {round}"
                );
            } else {
                if round == expected_round {
                    assert_eq!(
                        prev, expected_prev,
                        "unexpected prev_block_hash in trace message"
                    );
                }
                round_prev.insert(round, prev);
            }
        }
        _ => {}
    }
}

fn write_failure_trace(
    network_seed: u64,
    nodes: usize,
    trace: &[TraceEntry],
) -> PathBuf {
    let path = if let Ok(path) = std::env::var("DUSK_CONSENSUS_TRACE_FAIL_OUT")
    {
        PathBuf::from(path)
    } else {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_millis();
        path.push(format!(
            "dusk-consensus-fault-trace-{network_seed}-{stamp}.log"
        ));
        path
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let meta = TraceMeta {
        seed: Some(network_seed),
        nodes: Some(nodes),
    };
    write_trace_with_meta(&path, trace, &meta);
    path
}

async fn record_round_trace(
    tip_height: u64,
    tip_hash: Option<[u8; 32]>,
    timeouts: TimeoutSet,
) -> (Vec<TraceEntry>, u32) {
    let mut network = TestNetwork::new(TRACE_NETWORK_SIZE, TRACE_NETWORK_SEED);
    network.tip_header.height = tip_height;
    if let Some(hash) = tip_hash {
        network.tip_header.hash = hash;
    }

    let (cancels, handles) = spawn_all(&network, timeouts.clone());
    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut trace: Vec<TraceEntry> = Vec::new();
    let mut pending: Vec<(support::Envelope, u32)> = Vec::new();
    let mut quorum = None;
    let mut tick: u32 = 0;

    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        for env in batch {
            let delay = (env.from as u32 + env.msg.get_step() as u32) % 3;
            let deliver_at = tick + delay;
            trace.push(TraceEntry {
                from: env.from,
                deliver_at,
                payload_hex: encode_message(&env.msg),
            });
            pending.push((env, deliver_at));
        }

        let mut delivery = Vec::new();
        pending.retain(|(env, at)| {
            if *at <= tick {
                delivery.push(env.clone());
                false
            } else {
                true
            }
        });

        if find_quorum(&delivery).is_some() {
            quorum = Some(());
            break;
        }

        if !delivery.is_empty() {
            deliver_all(&network.nodes, &delivery);
        }

        tick = tick.wrapping_add(1);
    }

    shutdown_all(cancels, handles).await;
    router.stop();

    assert!(
        quorum.is_some(),
        "expected quorum for round {}",
        tip_height + 1
    );
    let max_tick = trace
        .iter()
        .map(|entry| entry.deliver_at)
        .max()
        .unwrap_or(0);
    (trace, max_tick)
}

#[test]
fn committee_is_deterministic_for_same_inputs() {
    let network = TestNetwork::new(5, 42);
    let provisioners = network.provisioners.clone();
    let seed = network.tip_header.seed;
    let round = 1;
    let iter = 0;

    let cfg = SortitionConfig::new(seed, round, iter, StepName::Validation, vec![]);
    let c1 = Committee::new(&provisioners, &cfg);
    let c2 = Committee::new(&provisioners, &cfg);

    assert_eq!(c1.members(), c2.members());
}

#[test]
fn committees_exclude_generators_on_non_proposal_steps() {
    let network = TestNetwork::new(6, 43);
    let seed = network.tip_header.seed;
    let round = network.tip_header.height + 1;
    let iter = 0;

    let current_generator =
        network.provisioners.get_generator(iter, seed, round);
    let next_generator =
        network.provisioners.get_generator(iter + 1, seed, round);

    let proposal_committee =
        build_committee(&network, iter, StepName::Proposal);
    assert!(
        proposal_committee
            .iter()
            .any(|pk| pk.bytes() == &current_generator),
        "proposal committee should include current generator"
    );

    for step in [StepName::Validation, StepName::Ratification] {
        let committee = build_committee(&network, iter, step);
        assert!(
            committee.excluded().contains(&current_generator),
            "current generator should be excluded in {step:?}"
        );
        assert!(
            !committee
                .iter()
                .any(|pk| pk.bytes() == &current_generator),
            "current generator should not be in {step:?} committee"
        );

        if dusk_consensus::config::exclude_next_generator(iter) {
            assert!(
                committee.excluded().contains(&next_generator),
                "next generator should be excluded in {step:?}"
            );
            assert!(
                !committee.iter().any(|pk| pk.bytes() == &next_generator),
                "next generator should not be in {step:?} committee"
            );
        }
    }
}

#[tokio::test]
async fn single_node_without_peers_does_not_reach_quorum() {
    let network = TestNetwork::new(1, 100);
    let timeouts = TestNetwork::fast_timeouts();

    let node = &network.nodes[0];
    let (cancels, handles) = spawn_all(&network, timeouts);

    let msg = wait_for_quorum(&node.outbound, Duration::from_secs(2)).await;
    assert!(msg.is_none(), "single node should not reach quorum");

    let stored = node.db.lock().await.candidates.len();
    assert!(stored > 0, "candidate should be stored locally");

    shutdown_all(cancels, handles).await;

}

#[tokio::test]
async fn multi_nodes_propagate_candidate_and_reach_quorum() {
    let network = TestNetwork::new(3, 77);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        assert_quorum_batch_invariants_with_network(
            &batch,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&batch) {
            quorum = Some(q);
            break;
        }
        deliver_all(&network.nodes, &batch);
    }

    let msg = quorum.expect("expected quorum message from either node");
    assert_quorum_message_invariants(&msg);
    assert_quorum_message_verifies(&network, &msg);

    match msg.payload {
        Payload::Quorum(q) => match q.att.result {
            RatificationResult::Success(_) => {}
            other => panic!("unexpected quorum result: {other:?}"),
        },
        _ => panic!("expected quorum payload"),
    }

    // Ensure non-generator stored a candidate (candidate propagated)
    let generator = network
        .provisioners
        .get_generator(0, network.tip_header.seed, 1);
    let non_generator = network
        .nodes
        .iter()
        .find(|n| n.keys.pk.bytes() != &generator)
        .expect("non-generator node");

    let stored = non_generator.db.lock().await.candidates.len();
    assert!(stored > 0, "non-generator should store candidate");

    shutdown_all(cancels, handles).await;

    router.stop();
}

#[tokio::test]
async fn shuffled_delivery_reaches_quorum() {
    let network = TestNetwork::new(3, 200);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let mut rng = StdRng::seed_from_u64(777);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let mut batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        batch.shuffle(&mut rng);
        assert_quorum_batch_invariants_with_network(
            &batch,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&batch) {
            quorum = Some(q);
            break;
        }
        deliver_all(&network.nodes, &batch);
    }

    assert!(quorum.is_some(), "expected quorum under shuffled delivery");
    assert_quorum_message_invariants(quorum.as_ref().unwrap());
    assert_quorum_message_verifies(&network, quorum.as_ref().unwrap());

    shutdown_all(cancels, handles).await;

    router.stop();
}

#[tokio::test]
async fn duplicate_delivery_does_not_break_quorum() {
    let network = TestNetwork::new(3, 300);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let mut rng = StdRng::seed_from_u64(888);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let mut batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        batch.shuffle(&mut rng);
        let mut delivery = batch.clone();
        for env in batch.iter().take(3) {
            delivery.push(env.clone());
        }
        assert_quorum_batch_invariants_with_network(
            &delivery,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&delivery) {
            quorum = Some(q);
            break;
        }
        deliver_all(&network.nodes, &delivery);
    }

    assert!(quorum.is_some(), "expected quorum with duplicates");
    assert_quorum_message_invariants(quorum.as_ref().unwrap());
    assert_quorum_message_verifies(&network, quorum.as_ref().unwrap());

    shutdown_all(cancels, handles).await;

    router.stop();
}

#[tokio::test]
async fn corrupted_messages_do_not_break_quorum() {
    let network = TestNetwork::new(3, 650);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        assert_quorum_batch_invariants_with_network(
            &batch,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&batch) {
            quorum = Some(q);
            break;
        }
        let mut delivery = batch.clone();
        for env in &batch {
            if let Some(mutated) = corrupt_message_signature(&env.msg) {
                delivery.push(support::Envelope {
                    from: env.from,
                    msg: mutated,
                });
            }
        }
        deliver_all(&network.nodes, &delivery);
    }

    assert!(quorum.is_some(), "expected quorum with corrupted extras");
    assert_quorum_message_invariants(quorum.as_ref().unwrap());
    assert_quorum_message_verifies(&network, quorum.as_ref().unwrap());

    shutdown_all(cancels, handles).await;

    router.stop();
}

#[tokio::test]
async fn network_partition_prevents_quorum() {
    let network = TestNetwork::new(4, 700);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let partition_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    let mut saw_quorum = false;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < partition_deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        assert_quorum_batch_invariants_with_network(
            &batch,
            &mut seen_quorums,
            &network,
        );
        if find_quorum(&batch).is_some() {
            saw_quorum = true;
            break;
        }
        for env in &batch {
            let group = if env.from < 2 { 0 } else { 1 };
            for (idx, node) in network.nodes.iter().enumerate() {
                if idx == env.from {
                    continue;
                }
                let peer_group = if idx < 2 { 0 } else { 1 };
                if peer_group == group {
                    node.inbound.try_send(env.msg.clone());
                }
            }
        }
    }

    assert!(!saw_quorum, "quorum should not be reached during partition");
    router.stop();

    shutdown_all(cancels, handles).await;
}

#[tokio::test]
async fn deterministic_schedule_reaches_quorum() {
    let network = TestNetwork::new(4, 800);
    let timeouts = TestNetwork::base_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(7);
    let mut pending: Vec<(support::Envelope, u8)> = Vec::new();
    let mut quorum = None;
    let mut tick = 0u8;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let mut batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch.is_empty() {
            continue;
        }
        batch.sort_by_key(|env| {
            (
                env.from,
                env.msg.header.round,
                env.msg.header.iteration,
                env.msg.get_step(),
            )
        });

        for (idx, env) in batch.into_iter().enumerate() {
            let delay = (tick.wrapping_add(idx as u8)) % 3;
            pending.push((env, delay));
        }

        let mut delivery = Vec::new();
        for item in pending.iter_mut() {
            if item.1 == 0 {
                delivery.push(item.0.clone());
            } else {
                item.1 -= 1;
            }
        }
        pending.retain(|item| item.1 != 0);

        assert_quorum_batch_invariants_with_network(
            &delivery,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&delivery) {
            quorum = Some(q);
            break;
        }
        deliver_all(&network.nodes, &delivery);
        tick = tick.wrapping_add(1);
    }

    assert!(quorum.is_some(), "expected quorum under deterministic schedule");
    assert_quorum_message_invariants(quorum.as_ref().unwrap());
    assert_quorum_message_verifies(&network, quorum.as_ref().unwrap());

    shutdown_all(cancels, handles).await;

    router.stop();
}

#[tokio::test]
async fn emergency_mode_repropagates_past_iteration_message() {
    let network = TestNetwork::new(4, 1400);
    set_emergency_iteration(&network).await;
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());
    let router = BufferedRouter::start(&network.nodes);

    let iter = EMERGENCY_MODE_ITERATION_THRESHOLD;
    let round = network.tip_header.height + 1;
    let generator =
        network.provisioners.get_generator(iter, network.tip_header.seed, round);
    let gen_idx = network
        .nodes
        .iter()
        .position(|node| node.keys.pk.bytes() == &generator)
        .expect("generator node");
    let generator_ru =
        network.nodes[gen_idx].round_update(&network.tip_header, timeouts);
    let msg = build_candidate_message(&generator_ru, iter);
    let msg_hex = encode_message(&msg);

    network.nodes[0].inbound.try_send(msg);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut saw_repropagation = false;
    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        if batch
            .iter()
            .any(|env| encode_message(&env.msg) == msg_hex)
        {
            saw_repropagation = true;
            break;
        }
    }

    assert!(
        saw_repropagation,
        "past iteration message should be rebroadcast in emergency mode"
    );

    shutdown_all(cancels, handles).await;
    router.stop();
}

#[tokio::test]
async fn emergency_mode_requests_missing_resources_on_timeouts() {
    let network = TestNetwork::new(4, 1410);
    set_emergency_iteration(&network).await;
    let mut timeouts = HashMap::new();
    timeouts.insert(StepName::Proposal, Duration::from_millis(60));
    timeouts.insert(StepName::Validation, Duration::from_millis(90));
    timeouts.insert(StepName::Ratification, Duration::from_millis(120));

    let (cancels, handles) = spawn_all(&network, timeouts);
    let router = BufferedRouter::start(&network.nodes);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_candidate_request = false;
    let mut saw_validation_request = false;

    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        for env in &batch {
            if let Payload::GetResource(req) = &env.msg.payload {
                for inv in &req.get_inv().inv_list {
                    match inv.inv_type {
                        InvType::CandidateFromIteration => {
                            saw_candidate_request = true;
                        }
                        InvType::ValidationResult => {
                            saw_validation_request = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        if saw_candidate_request && saw_validation_request {
            break;
        }
        // Drop messages to force proposal/validation timeouts in emergency mode.
    }

    assert!(
        saw_candidate_request,
        "emergency mode should request candidate on timeout"
    );
    assert!(
        saw_validation_request,
        "emergency mode should request validation result on timeout"
    );

    shutdown_all(cancels, handles).await;
    router.stop();
}

#[tokio::test]
async fn far_future_round_message_is_dropped() {
    let network = TestNetwork::new(3, 1300);
    let timeouts = TestNetwork::fast_timeouts();
    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let node = &network.nodes[0];
    let current_round = network.tip_header.height + 1;
    let far_round = current_round + MAX_ROUND_DISTANCE + 1;
    let mut far_header = network.tip_header.clone();
    far_header.height = far_round - 1;
    far_header.hash = [9u8; 32];
    let ru_far = node.round_update(&far_header, timeouts.clone());
    let validation =
        build_validation_payload(Vote::NoCandidate, &ru_far, 0);
    let msg: Message = validation.into();
    let msg_hex = encode_message(&msg);
    node.inbound.try_send(msg);

    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    let mut forwarded = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(50), node.outbound.recv()).await {
            Ok(Ok(out)) => {
                if encode_message(&out) == msg_hex {
                    forwarded = true;
                    break;
                }
            }
            _ => {}
        }
    }

    let queued = node
        .future_msgs
        .lock()
        .await
        .drain_msg_by_round_step(
            far_round,
            StepName::Validation.to_step(0),
        )
        .map(|items| items.len())
        .unwrap_or(0);

    assert!(
        !forwarded,
        "far-future message should not be rebroadcast"
    );
    assert_eq!(
        queued, 0,
        "far-future message should not be queued"
    );

    shutdown_all(cancels, handles).await;
}

#[tokio::test]
async fn seeded_fault_injection_schedules_reach_quorum() {
    for seed in 1u64..=3 {
        if let Err(trace_path) = run_fault_injection_schedule(seed).await {
            panic!(
                "seed {seed} failed to reach quorum; trace written to {:?}",
                trace_path
            );
        }
    }
}

async fn run_fault_injection_schedule(seed: u64) -> Result<(), PathBuf> {
    let network_seed = 1100 + seed;
    let network = TestNetwork::new(3, network_seed);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut pending: Vec<(support::Envelope, u32)> = Vec::new();
    let mut trace: Vec<TraceEntry> = Vec::new();
    let mut tick: u32 = 0;
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let mut batch = router.recv_batch(Duration::from_millis(120)).await;
        assert_quorum_batch_invariants_with_network(
            &batch,
            &mut seen_quorums,
            &network,
        );
        if find_quorum(&batch).is_some() {
            quorum = Some(());
            break;
        }

        batch.shuffle(&mut rng);

        for env in batch {
            if matches!(env.msg.payload, Payload::Candidate(_)) {
                let delay = rng.gen_range(0..=2);
                let deliver_at = tick + delay;
                let payload_hex = encode_message(&env.msg);
                trace.push(TraceEntry {
                    from: env.from,
                    deliver_at,
                    payload_hex: payload_hex.clone(),
                });
                pending.push((env.clone(), deliver_at));
                if rng.gen_bool(0.1) {
                    let deliver_at = tick + delay + 1;
                    trace.push(TraceEntry {
                        from: env.from,
                        deliver_at,
                        payload_hex: payload_hex.clone(),
                    });
                    pending.push((env, deliver_at));
                }
                continue;
            }

            let roll: u8 = rng.gen_range(0..100);
            if roll < 2 {
                continue; // drop
            }

            let mut to_send = env.clone();
            if roll >= 2 && roll < 4 {
                if let Some(mutated) = corrupt_message_signature(&env.msg) {
                    to_send.msg = mutated;
                }
            }

            let delay = rng.gen_range(0..=2);
            let deliver_at = tick + delay;
            trace.push(TraceEntry {
                from: to_send.from,
                deliver_at,
                payload_hex: encode_message(&to_send.msg),
            });
            pending.push((to_send.clone(), deliver_at));

            if roll >= 4 && roll < 9 {
                let deliver_at = tick + delay + 1;
                trace.push(TraceEntry {
                    from: to_send.from,
                    deliver_at,
                    payload_hex: encode_message(&to_send.msg),
                });
                pending.push((to_send, deliver_at));
            }
        }

        let mut delivery = Vec::new();
        pending.retain(|(env, at)| {
            if *at <= tick {
                delivery.push(env.clone());
                false
            } else {
                true
            }
        });

        if !delivery.is_empty() {
            deliver_all(&network.nodes, &delivery);
        }

        tick = tick.wrapping_add(1);
    }

    shutdown_all(cancels, handles).await;
    router.stop();

    if quorum.is_some() {
        Ok(())
    } else {
        Err(write_failure_trace(network_seed, network.nodes.len(), &trace))
    }
}

async fn replay_trace_entries(
    trace: &[TraceEntry],
    network_size: usize,
    seed: u64,
    timeouts: dusk_consensus::commons::TimeoutSet,
) -> bool {
    let replay_network = TestNetwork::new(network_size, seed);
    let (cancels, handles) = spawn_all(&replay_network, timeouts);

    let router = BufferedRouter::start(&replay_network.nodes);
    let expected_round = replay_network.tip_header.height + 1;
    let expected_prev = replay_network.tip_header.hash;
    let max_tick = trace
        .iter()
        .map(|entry| entry.deliver_at)
        .max()
        .unwrap_or(0);
    let mut quorum = None;
    let mut seen_quorums = HashMap::new();
    let mut seen_trace_quorums = HashMap::new();
    let mut trace_round_prev: HashMap<u64, [u8; 32]> = HashMap::new();

    for tick in 0..=max_tick + 2 {
        let mut delivery = Vec::new();
        for entry in trace.iter().filter(|e| e.deliver_at == tick) {
            let msg = decode_message(&entry.payload_hex);
            track_trace_round_prev(
                &msg,
                expected_round,
                expected_prev,
                &mut trace_round_prev,
            );
            if matches!(msg.payload, Payload::Quorum(_)) {
                assert_quorum_message_invariants(&msg);
                assert_no_conflicting_quorums(
                    &[support::Envelope {
                        from: entry.from,
                        msg: msg.clone(),
                    }],
                    &mut seen_trace_quorums,
                );
            }
            delivery.push(support::Envelope {
                from: entry.from,
                msg,
            });
        }
        if !delivery.is_empty() {
            deliver_all(&replay_network.nodes, &delivery);
        }

        let batch = router.recv_batch(Duration::from_millis(200)).await;
        assert_quorum_batch_invariants_with_network_for_round(
            &batch,
            &mut seen_quorums,
            &replay_network,
            Some(expected_round),
        );
        let round_quorum = batch.iter().find_map(|env| {
            if let Payload::Quorum(q) = &env.msg.payload {
                if q.header.round == expected_round {
                    return Some(env.msg.clone());
                }
            }
            None
        });
        if let Some(q) = round_quorum {
            quorum = Some(q);
            break;
        }
    }

    shutdown_all(cancels, handles).await;
    router.stop();
    if let Some(ref msg) = quorum {
        assert_quorum_message_invariants(msg);
        assert_quorum_message_verifies(&replay_network, msg);
    }
    quorum.is_some()
}

#[tokio::test]
async fn replay_fault_injection_trace_from_env() {
    let path = match std::env::var("DUSK_CONSENSUS_TRACE_REPLAY") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };

    let (meta, trace) = read_trace_with_meta(&path);
    assert!(!trace.is_empty(), "trace is empty: {:?}", path);

    let nodes = meta.nodes.unwrap_or_else(|| {
        trace
            .iter()
            .map(|entry| entry.from)
            .max()
            .unwrap_or(0)
            + 1
    });
    let seed = match meta.seed {
        Some(seed) => seed,
        None => panic!("trace missing seed metadata: {:?}", path),
    };

    let ok = replay_trace_entries(&trace, nodes, seed, TestNetwork::fast_timeouts())
        .await;
    assert!(ok, "expected quorum when replaying trace {:?}", path);
}

#[tokio::test]
async fn record_replay_trace_reaches_quorum() {
    let network = TestNetwork::new(TRACE_NETWORK_SIZE, TRACE_NETWORK_SEED);
    let timeouts = TestNetwork::fast_timeouts();

    let (cancels, handles) = spawn_all(&network, timeouts.clone());

    let router = BufferedRouter::start(&network.nodes);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut trace: Vec<TraceEntry> = Vec::new();
    let mut pending: Vec<(support::Envelope, u32)> = Vec::new();
    let mut quorum = None;
    let mut tick: u32 = 0;
    let mut seen_quorums = HashMap::new();

    while tokio::time::Instant::now() < deadline {
        let batch = router.recv_batch(Duration::from_millis(200)).await;
        for env in batch {
            let delay = (env.from as u32 + env.msg.get_step() as u32) % 3;
            let deliver_at = tick + delay;
            trace.push(TraceEntry {
                from: env.from,
                deliver_at,
                payload_hex: encode_message(&env.msg),
            });
            pending.push((env, deliver_at));
        }

        let mut delivery = Vec::new();
        pending.retain(|(env, at)| {
            if *at <= tick {
                delivery.push(env.clone());
                false
            } else {
                true
            }
        });

        assert_quorum_batch_invariants_with_network(
            &delivery,
            &mut seen_quorums,
            &network,
        );
        if let Some(q) = find_quorum(&delivery) {
            quorum = Some(q);
            break;
        }
        if !delivery.is_empty() {
            deliver_all(&network.nodes, &delivery);
        }
        tick = tick.wrapping_add(1);
    }

    assert!(quorum.is_some(), "expected quorum in record phase");
    assert_quorum_message_invariants(quorum.as_ref().unwrap());
    assert_quorum_message_verifies(&network, quorum.as_ref().unwrap());

    let trace_path = if let Ok(path) =
        std::env::var("DUSK_CONSENSUS_TRACE_OUT")
    {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("trace dir");
        }
        path
    } else {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_millis();
        path.push(format!("dusk-consensus-trace-{stamp}.log"));
        path
    };
    let meta = TraceMeta {
        seed: Some(TRACE_NETWORK_SEED),
        nodes: Some(TRACE_NETWORK_SIZE),
    };
    write_trace_with_meta(&trace_path, &trace, &meta);

    shutdown_all(cancels, handles).await;

    router.stop();

    let (meta, replay) = read_trace_with_meta(&trace_path);
    assert_eq!(meta.seed, Some(TRACE_NETWORK_SEED));
    assert_eq!(meta.nodes, Some(TRACE_NETWORK_SIZE));
    let ok =
        replay_trace_entries(&replay, TRACE_NETWORK_SIZE, TRACE_NETWORK_SEED, timeouts.clone())
            .await;
    assert!(ok, "expected quorum in replay phase");
    if std::env::var("DUSK_CONSENSUS_TRACE_OUT").is_err() {
        let _ = std::fs::remove_file(&trace_path);
    }
}

#[tokio::test]
#[ignore = "manual trace generation"]
async fn record_multi_round_trace() {
    let path = match std::env::var("DUSK_CONSENSUS_TRACE_MULTI_OUT") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };

    let timeouts = TestNetwork::fast_timeouts();
    let (mut trace, max_tick) =
        record_round_trace(0, None, timeouts.clone()).await;
    let (mut round_two, _) = record_round_trace(1, Some([9u8; 32]), timeouts).await;
    let offset = max_tick.saturating_add(5);
    for entry in &mut round_two {
        entry.deliver_at = entry.deliver_at.saturating_add(offset);
    }
    trace.append(&mut round_two);

    let meta = TraceMeta {
        seed: Some(TRACE_NETWORK_SEED),
        nodes: Some(TRACE_NETWORK_SIZE),
    };
    write_trace_with_meta(&path, &trace, &meta);
}

#[tokio::test]
async fn replay_canonical_trace_reaches_quorum() {
    let trace_path = canonical_trace_path();
    let (meta, trace) = read_trace_with_meta(&trace_path);
    assert!(
        !trace.is_empty(),
        "canonical trace is empty: {:?}",
        trace_path
    );
    assert_eq!(meta.seed, Some(TRACE_NETWORK_SEED));
    assert_eq!(meta.nodes, Some(TRACE_NETWORK_SIZE));
    let ok = replay_trace_entries(
        &trace,
        TRACE_NETWORK_SIZE,
        TRACE_NETWORK_SEED,
        TestNetwork::base_timeouts(),
    )
    .await;
    assert!(ok, "expected quorum in canonical replay");
}

#[tokio::test]
async fn replay_multi_round_trace_reaches_quorum() {
    let trace_path = multi_round_trace_path();
    let (meta, trace) = read_trace_with_meta(&trace_path);
    assert!(
        !trace.is_empty(),
        "multi-round trace is empty: {:?}",
        trace_path
    );
    assert_eq!(meta.seed, Some(TRACE_NETWORK_SEED));
    assert_eq!(meta.nodes, Some(TRACE_NETWORK_SIZE));
    let ok = replay_trace_entries(
        &trace,
        TRACE_NETWORK_SIZE,
        TRACE_NETWORK_SEED,
        TestNetwork::base_timeouts(),
    )
    .await;
    assert!(ok, "expected quorum in multi-round replay");
}

#[tokio::test]
#[ignore = "long-run randomized schedule; run manually"]
async fn random_schedules_eventually_reach_quorum() {
    for seed in 1u64..=3 {
        let network = TestNetwork::new(6, 500 + seed);
        let timeouts = TestNetwork::base_timeouts();

        let (cancels, handles) = spawn_all(&network, timeouts.clone());

        let router = BufferedRouter::start(&network.nodes);
        let mut rng = StdRng::seed_from_u64(900 + seed);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let mut quorum = None;

        while tokio::time::Instant::now() < deadline {
            let mut batch = router.recv_batch(Duration::from_millis(200)).await;
            if batch.is_empty() {
                continue;
            }
            batch.shuffle(&mut rng);
            if let Some(q) = find_quorum(&batch) {
                quorum = Some(q);
                break;
            }
            deliver_all(&network.nodes, &batch);
        }

        assert!(
            quorum.is_some(),
            "expected quorum under random schedule (seed {seed})"
        );

        shutdown_all(cancels, handles).await;

        router.stop();
    }
}
