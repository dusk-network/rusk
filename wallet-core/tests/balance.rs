// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

use execution_core::{
    transfer::phoenix::{
        Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    },
    JubJubScalar,
};

use wallet_core::{phoenix_balance, BalanceInfo};

#[test]
fn test_balance() {
    let mut rng = StdRng::seed_from_u64(0xdab);

    let owner_sk = PhoenixSecretKey::random(&mut rng);
    let owner_pk = PhoenixPublicKey::from(&owner_sk);
    let sender_pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

    let mut notes = Vec::new();

    // create the notes
    for value in 0..=10 {
        let value_blinder = JubJubScalar::random(&mut rng);
        let sender_blinder = [
            JubJubScalar::random(&mut rng),
            JubJubScalar::random(&mut rng),
        ];

        // we want to test with a mix of transparent and obfuscated notes so we
        // make every 10th note transparent
        let note = if value % 10 == 0 {
            Note::transparent(
                &mut rng,
                &sender_pk,
                &owner_pk,
                value,
                sender_blinder,
            )
        } else {
            Note::obfuscated(
                &mut rng,
                &sender_pk,
                &owner_pk,
                value,
                value_blinder,
                sender_blinder,
            )
        };
        notes.push(note);
    }

    // the sum of these notes should be 5 * 11 = 55
    // and the spendable notes are 7 + 8 + 9 + 10 = 34
    let expected_balance = BalanceInfo {
        value: 55,
        spendable: 34,
    };

    assert_eq!(
        phoenix_balance(&(&owner_sk).into(), notes),
        expected_balance
    );
}
