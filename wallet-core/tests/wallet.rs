// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

mod mock;

use dusk_bytes::Serializable;
use dusk_plonk::prelude::BlsScalar;
use dusk_wallet_core::StakeInfo;
use mock::{mock_canon_wallet, mock_serde_wallet, mock_wallet};

#[test]
fn serde_stake() {
    let stake = StakeInfo {
        amount: Some((1000, 0)),
        reward: 100,
        counter: 1,
    };

    let stake_bytes = stake.to_bytes();
    let des_stake =
        StakeInfo::from_bytes(&stake_bytes).expect("serde to go correctly");

    assert_eq!(stake.amount, des_stake.amount);
    assert_eq!(stake.reward, des_stake.reward);
    assert_eq!(stake.counter, des_stake.counter);
}

#[test]
fn serde() {
    let mut rng = rand::thread_rng();

    let wallet = mock_serde_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}

#[test]
fn canon() {
    let mut rng = rand::thread_rng();

    let wallet = mock_canon_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}

#[test]
fn transfer() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}

#[test]
fn get_balance() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 5000, 2500, 5000, 5000]);
    let info = wallet.get_balance(0).expect("Valid balance call");

    assert_eq!(info.value, 20000);
    assert_eq!(info.spendable, 17500);
}
