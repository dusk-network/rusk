// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::time::Duration;

use dusk_consensus::commons::RoundUpdate;
use node::chain;

use criterion::async_executor::FuturesExecutor;
use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};

use bls12_381_bls::{
    PublicKey as StakePublicKey, SecretKey as StakeSecretKey,
    Signature as StakeSignature,
};
use dusk_bytes::Serializable;
use dusk_consensus::user::{
    cluster::Cluster, committee::Committee, provisioners::Provisioners,
    sortition::Config as SortitionConfig,
};
use node_data::ledger::{Certificate, StepVotes};
use node_data::message::payload::{
    QuorumType, RatificationResult, ValidationResult, Vote,
};
use node_data::{ledger, StepName};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn create_step_votes(
    mrb_header: &ledger::Header,
    vote: &Vote,
    step: StepName,
    iteration: u8,
    provisioners: &Provisioners,
    keys: &[(node_data::bls::PublicKey, StakeSecretKey)],
) -> StepVotes {
    let round = mrb_header.height + 1;
    let seed = mrb_header.seed;

    let generator = provisioners.get_generator(iteration, seed, round);

    let sortition_config =
        SortitionConfig::new(seed, round, iteration, step, Some(generator));

    let committee = Committee::new(provisioners, &sortition_config);

    let mut signatures = vec![];
    let mut cluster = Cluster::<node_data::bls::PublicKey>::default();
    for (pk, sk) in keys.iter() {
        if let Some(weight) = committee.votes_for(pk) {
            let vote = vote.clone();
            let ru = RoundUpdate::new(
                pk.clone(),
                *sk,
                mrb_header,
                HashMap::default(),
            );
            let sig = match step {
                StepName::Validation => {
                    dusk_consensus::build_validation_payload(
                        vote, &ru, iteration,
                    )
                    .sign_info
                    .signature
                }
                StepName::Ratification => {
                    dusk_consensus::build_ratification_payload(
                        &ru,
                        iteration,
                        &ValidationResult::new(
                            StepVotes::default(),
                            vote,
                            QuorumType::Valid,
                        ),
                    )
                    .sign_info
                    .signature
                }
                _ => unreachable!(),
            };
            signatures.push(StakeSignature::from_bytes(sig.inner()).unwrap());
            cluster.set_weight(pk, weight);
        }
    }

    let bitset = committee.bits(&cluster);

    let (first, rest) = signatures.split_first().unwrap();
    let aggregate_signature = first.aggregate(rest).to_bytes();
    StepVotes::new(aggregate_signature, bitset)
}

pub fn with_group<T, F>(name: &str, c: &mut Criterion, closure: F) -> T
where
    F: FnOnce(&mut BenchmarkGroup<WallTime>) -> T,
{
    let mut group = c.benchmark_group(name);
    let r = closure(&mut group);
    group.finish();
    r
}

pub fn verify_block_cert(c: &mut Criterion) {
    with_group("verify_block_cert", c, |group| {
        for input in INPUTS {
            group.measurement_time(Duration::from_secs(input.measurement_time));
            let mut keys = vec![];
            let mut provisioners = Provisioners::empty();
            let rng = &mut StdRng::seed_from_u64(0xbeef);
            for _ in 0..input.provisioners {
                let sk = StakeSecretKey::random(rng);
                let pk = StakePublicKey::from(&sk);
                let pk = node_data::bls::PublicKey::new(pk);
                keys.push((pk.clone(), sk));
                provisioners.add_member_with_value(pk, 1000000000000)
            }
            let mrb_header = ledger::Header {
                seed: [5; 48].into(),
                ..Default::default()
            };
            let block_hash = [1; 32];
            let vote = Vote::Valid(block_hash);
            let iteration = 0;

            let validation = create_step_votes(
                &mrb_header,
                &vote,
                StepName::Validation,
                iteration,
                &provisioners,
                &keys[..],
            );
            let ratification = create_step_votes(
                &mrb_header,
                &vote,
                StepName::Ratification,
                iteration,
                &provisioners,
                &keys[..],
            );
            let cert = Certificate {
                result: RatificationResult::Success(Vote::Valid(block_hash)),
                validation,
                ratification,
            };

            group.bench_function(
                BenchmarkId::new(
                    "verify_block_cert",
                    format!("{} prov", input.provisioners),
                ),
                move |b| {
                    b.to_async(FuturesExecutor).iter(|| async {
                        chain::verify_block_cert(
                            [0u8; 32],
                            mrb_header.seed,
                            &provisioners,
                            mrb_header.height + 1,
                            &cert,
                            iteration,
                        )
                        .await
                        .expect("block to be verified")
                    })
                },
            );
        }
    })
}

struct Input {
    provisioners: usize,
    measurement_time: u64, // secs
}

const INPUTS: &[Input] = &[
    Input {
        provisioners: 1,
        measurement_time: 10,
    },
    Input {
        provisioners: 30,
        measurement_time: 10,
    },
    Input {
        provisioners: 64,
        measurement_time: 10,
    },
    Input {
        provisioners: 256,
        measurement_time: 15,
    },
    Input {
        provisioners: 1000,
        measurement_time: 15,
    },
];
criterion_group!(benches, verify_block_cert);
criterion_main!(benches);
