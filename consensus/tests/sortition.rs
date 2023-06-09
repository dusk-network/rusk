// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::user::committee::Committee;
use dusk_consensus::user::provisioners::{Provisioners, DUSK};
use dusk_consensus::user::sortition::Config;

use node_data::bls::PublicKey;

use node_data::ledger::Seed;

#[test]
fn test_deterministic_sortition_1() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let committee_size = 64;

    // Execute sortition with specific config
    let cfg = Config::new(Seed::default(), 1, 1, 64);
    p.update_eligibility_flag(cfg.round);

    let committee = Committee::new(PublicKey::default(), &mut p, cfg);

    assert_eq!(
        committee_size,
        committee.get_occurrences().iter().sum::<usize>()
    );
    assert_eq!(vec![7, 23, 13, 21], committee.get_occurrences());
}

#[test]
fn test_deterministic_sortition_2() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let committee_size = 45;
    let cfg = Config::new(Seed::from([3u8; 48]), 7777, 8, committee_size);

    let committee = Committee::new(PublicKey::default(), &mut p, cfg);
    assert_eq!(
        committee_size,
        committee.get_occurrences().iter().sum::<usize>()
    );
    assert_eq!(vec![5, 13, 14, 13], committee.get_occurrences());
}

#[test]
fn test_quorum() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let cfg = Config::new(Seed::default(), 7777, 8, 64);
    p.update_eligibility_flag(cfg.round);

    let c = Committee::new(PublicKey::default(), &mut p, cfg);
    assert_eq!(c.quorum(), 43);
}

#[test]
fn test_intersect() {
    let mut p = generate_provisioners(10);

    let cfg = Config::new(Seed::default(), 1, 3, 200);
    p.update_eligibility_flag(cfg.round);
    // println!("{:#?}", p);

    let c = Committee::new(PublicKey::default(), &mut p, cfg);
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
    let mut p = Provisioners::new();
    for i in 1..n {
        let stake_value = 1000 * (i as u64) * DUSK;
        p.add_member_with_value(
            PublicKey::from_sk_seed_u64(i as u64),
            stake_value,
        );
    }
    p
}
