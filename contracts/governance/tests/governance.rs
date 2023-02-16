// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_pki::{PublicKey, SecretKey};
use executor::Executor;
use governance_contract::{GovernanceContract, Transfer};
use microkelvin::{BackendCtor, DiskBackend, Persistence};
use rand::rngs::StdRng;
use rand::SeedableRng;

mod executor;
mod tx;

fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

const DUMMY_TS: u64 = 946681200000; // Dummy timestamp representing 01/01/2000

#[test]
fn mint_burn_transfer() {
    Persistence::with_backend(&testbackend(), |_| Ok(()))
        .expect("Backend found");

    let mut rng = StdRng::seed_from_u64(0xbeef);
    let a = PublicKey::from(&SecretKey::random(&mut rng));
    let b = PublicKey::from(&SecretKey::random(&mut rng));

    let sk_authority = BlsSecretKey::random(&mut rng);

    let mut contract = GovernanceContract::default();
    // set authority
    contract.authority = BlsPublicKey::from(&sk_authority);

    let genesis_value = 100_000_000_000_000;

    let mut executor = Executor::new(2324, contract, genesis_value);
    let contract = executor.state();

    assert_eq!(contract.total_supply(), 0);
    assert_eq!(contract.balance(&a), 0);
    assert_eq!(contract.balance(&b), 0);

    let mint = tx::mint(&a, 100, &sk_authority);
    let contract = executor.run_tx(mint).expect("Failed to mint");

    assert_eq!(contract.total_supply(), 100);
    assert_eq!(contract.balance(&a), 100);
    assert_eq!(contract.balance(&b), 0);

    let t_1 = Transfer {
        // transfer 200 from a to b
        from: a,
        to: b,
        amount: 200,
        timestamp: DUMMY_TS,
    };
    let transfer_1 = tx::transfer(vec![t_1], &sk_authority);
    let contract = executor
        .run_tx(transfer_1)
        .expect("Failed to execute transfer 1");

    assert_eq!(contract.total_supply(), 200);
    assert_eq!(contract.balance(&a), 0);
    assert_eq!(contract.balance(&b), 200);

    let t_2 = Transfer {
        // transfer 50 from b to a
        from: b,
        to: a,
        amount: 50,
        timestamp: DUMMY_TS,
    };

    let transfer_2 = tx::transfer(vec![t_2], &sk_authority);
    let contract = executor
        .run_tx(transfer_2)
        .expect("Failed to execute transfer 2");

    assert_eq!(contract.total_supply(), 200);
    assert_eq!(contract.balance(&a), 50);
    assert_eq!(contract.balance(&b), 150);
}
