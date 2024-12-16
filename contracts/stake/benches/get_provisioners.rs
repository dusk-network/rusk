// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{criterion_group, criterion_main, Criterion};
use dusk_core::{
    stake::{StakeData, STAKE_CONTRACT},
    transfer::TRANSFER_CONTRACT,
    BlsPublicKey, BlsSecretKey,
};
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_abi::{ContractData, PiecrustError, Session, VM};
use std::sync::mpsc;

const SAMPLE_SIZE: usize = 10;
const NUM_STAKES: usize = 1000;

const OWNER: [u8; 32] = [0; 32];
const POINT_LIMIT: u64 = 0x100000000;
const TEST_STAKE: u64 = 500_000_000_000_000;

fn config() -> Criterion {
    Criterion::default().sample_size(SAMPLE_SIZE)
}

fn update_root(session: &mut Session) -> Result<(), PiecrustError> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

fn instantiate(vm: &VM) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let stake_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/stake_contract.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);

    session
        .deploy(
            transfer_bytecode,
            ContractData::builder(OWNER).contract_id(TRANSFER_CONTRACT),
            POINT_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy(
            stake_bytecode,
            ContractData::builder(OWNER).contract_id(STAKE_CONTRACT),
            POINT_LIMIT,
        )
        .expect("Deploying the stake contract should succeed");

    update_root(&mut session).expect("Updating the root should succeed");

    let base = session.commit().expect("Committing should succeed");

    rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed")
}

fn do_get_provisioners(
    session: &mut Session,
) -> Result<impl Iterator<Item = (BlsPublicKey, StakeData)>, PiecrustError> {
    let (sender, receiver) = mpsc::channel();
    session.feeder_call::<_, ()>(
        STAKE_CONTRACT,
        "stakes",
        &(),
        u64::MAX,
        sender,
    )?;
    Ok(receiver.into_iter().map(|bytes| {
        rkyv::from_bytes::<(BlsPublicKey, StakeData)>(&bytes)
            .expect("The contract should only return (pk, stake_data) tuples")
    }))
}

fn do_insert_stake<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    session: &mut Session,
) -> Result<(), PiecrustError> {
    let stake_data = StakeData {
        amount: Some((TEST_STAKE, 0)),
        nonce: 1,
        reward: 0,
    };
    let sk = BlsSecretKey::random(rng);
    let pk = BlsPublicKey::from(&sk);
    session.call::<_, ()>(
        STAKE_CONTRACT,
        "insert_stake",
        &(pk, stake_data),
        POINT_LIMIT,
    )?;
    Ok(())
}

fn get_provisioners(c: &mut Criterion) {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let mut session = instantiate(vm);

    for _ in 0..NUM_STAKES {
        do_insert_stake(rng, &mut session)
            .expect("inserting stake should succeed");
    }

    c.bench_function("get_provisioners", |b| {
        b.iter(|| {
            let _: Vec<(BlsPublicKey, StakeData)> =
                do_get_provisioners(&mut session)
                    .expect("getting provisioners should succeed")
                    .collect();
        });
    });
}

criterion_group!(
    name = benches;
    config = config();
    targets = get_provisioners
);
criterion_main!(benches);
