// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::{PublicKey as BlsPublicKey, SecretKey};
use dusk_bytes::DeserializableSlice;
use dusk_consensus::user::committee::Committee;
use dusk_consensus::user::provisioners::{Provisioners, DUSK};
use dusk_consensus::user::sortition::Config;

use node_data::bls::PublicKey;

use node_data::ledger::Seed;

#[test]
fn test_deterministic_sortition_1() {
    // Create provisioners with bls keys read from an external file.
    let p = generate_provisioners(5);

    let committee_size = 64;

    // Execute sortition with specific config
    let cfg = Config::new(Seed::default(), 1, 1, 64, None);

    let committee = Committee::new(PublicKey::default(), &p, &cfg);

    // Verify expected committee size
    assert_eq!(
        committee_size,
        committee.get_occurrences().iter().sum::<usize>()
    );

    // Verify expected distribution
    assert_eq!(vec![7, 32, 7, 18], committee.get_occurrences());
}

#[test]
fn test_deterministic_sortition_2() {
    // Create provisioners with bls keys read from an external file.
    let p = generate_provisioners(5);

    let committee_size = 45;
    let cfg = Config::new(Seed::from([3u8; 48]), 7777, 8, committee_size, None);

    let committee = Committee::new(PublicKey::default(), &p, &cfg);
    assert_eq!(
        committee_size,
        committee.get_occurrences().iter().sum::<usize>()
    );
    assert_eq!(vec![7, 15, 10, 13], committee.get_occurrences());
}

#[test]
fn test_quorum() {
    // Create provisioners with bls keys read from an external file.
    let p = generate_provisioners(5);

    let cfg = Config::new(Seed::default(), 7777, 8, 64, None);

    let c = Committee::new(PublicKey::default(), &p, &cfg);
    assert_eq!(c.quorum(), 43);
}

#[test]
fn test_intersect() {
    let p = generate_provisioners(10);

    let cfg = Config::new(Seed::default(), 1, 3, 200, None);
    // println!("{:#?}", p);

    let c = Committee::new(PublicKey::default(), &p, &cfg);
    // println!("{:#?}", c);

    let max_bitset = (2_i32.pow((c.size()) as u32) - 1) as u64;
    println!("max_bitset: {} / {:#064b} ", max_bitset, max_bitset);

    for bitset in 0..max_bitset {
        //println!("bitset: {:#064b}", bitset);
        let result = c.intersect(bitset);
        assert_eq!(c.bits(&result), bitset, "testing with  bitset:{}", bitset);
    }
}

fn generate_provisioners(n: usize) -> Provisioners {
    let sks = [
        "7f6f2ccdb23f2abb7b69278e947c01c6160a31cf02c19d06d0f6e5ab1d768b15",
        "611830d3641a68f94a690dcc25d1f4b0dac948325ac18f6dd32564371735f32c",
        "1fbec814b18b1d4c3eaa7cec41007e04bf0a98453b06ec7582aa29882c52eb3e",
        "ecd9c4a53ea15f18447b08fb96a13c5ab7dc7d24067b102fcbaaf7b39ca52e2d",
        "e463bcb1a6e57288ffd4671503082fa8656e3eacb78fb1925f8a7c76400e8e15",
        "7a19fb2d099a9557f7c10c2efbb8b101d9e0ec85610d5c74a887d1d4fb8d2827",
        "4dbad51eb408af559dd91bbbed8dbeae0a2c89e0e05f0cce87c98652a8437f1f",
        "befba86ae9e0c207865f7e24e8349d4ecdbc8b0f4632842499a0dfa60568e20a",
        "b260b8a10343bf5a5dacb4f1d32d06c4fdddc9981a3619fbc0a5cd9eb30f3334",
        "87a9779748888da5d96bbbce041b5109c6ffc0c4f30561c0170384a5922d9e21",
    ];
    let sks: Vec<_> = sks
        .iter()
        .take(n)
        .map(|hex| hex::decode(hex).expect("valid hex"))
        .map(|data| SecretKey::from_slice(&data[..]).expect("valid secret key"))
        .collect();

    let mut p = Provisioners::default();
    for (i, sk) in sks.iter().enumerate().skip(1) {
        let stake_value = 1000 * (i) as u64 * DUSK;
        let pubkey_bls = PublicKey::new(BlsPublicKey::from(sk));
        p.add_member_with_value(pubkey_bls, stake_value);
    }
    p
}
