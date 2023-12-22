// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use node::chain;

use criterion::async_executor::FuturesExecutor;
use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion,
};

use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    Signature as BlsSignature,
};
use dusk_bytes::Serializable;
use dusk_consensus::user::{
    cluster::Cluster, committee::Committee, provisioners::Provisioners,
    sortition::Config as SortitionConfig,
};
use node_data::{
    bls::PublicKey,
    ledger::{Certificate, Signature, StepVotes},
    message,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn create_step_votes(
    seed: Signature,
    round: u64,
    block_hash: [u8; 32],
    step: u8,
    iteration: u8,
    provisioners: &Provisioners,
    keys: &[(PublicKey, BlsSecretKey)],
) -> StepVotes {
    let sortition_config =
        SortitionConfig::new(seed, round, iteration * 3 + step, 64, None);

    let committee = Committee::new(provisioners, &sortition_config);

    let hdr = message::Header {
        round,
        step: step % 3,
        block_hash,
        ..Default::default()
    };
    let mut signatures = vec![];
    let mut cluster = Cluster::<PublicKey>::default();
    for (pk, sk) in keys.iter() {
        if let Some(weight) = committee.votes_for(pk) {
            let sig = hdr.sign(sk, pk.inner());
            signatures.push(BlsSignature::from_bytes(&sig).unwrap());
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
                let sk = BlsSecretKey::random(rng);
                let pk = BlsPublicKey::from(&sk);
                let pk = PublicKey::new(pk);
                keys.push((pk.clone(), sk));
                provisioners.add_member_with_value(pk, 1000000000000)
            }
            let height = 1;
            let seed = Signature([5; 48]);
            let block_hash = [1; 32];
            let iteration = 0;
            let mut cert = Certificate::default();

            cert.validation = create_step_votes(
                seed,
                height,
                block_hash,
                1,
                iteration,
                &provisioners,
                &keys[..],
            );
            cert.ratification = create_step_votes(
                seed,
                height,
                block_hash,
                2,
                iteration,
                &provisioners,
                &keys[..],
            );
            group.bench_function(
                BenchmarkId::new(
                    "verify_block_cert",
                    format!("{} prov", input.provisioners),
                ),
                move |b| {
                    b.to_async(FuturesExecutor).iter(|| {
                        chain::verify_block_cert(
                            seed,
                            &provisioners,
                            block_hash,
                            height,
                            &cert,
                            iteration,
                            true,
                        )
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
