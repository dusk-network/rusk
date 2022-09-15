// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use consensus::user::committee::Committee;
use consensus::user::provisioners::{Provisioners, DUSK};
use consensus::user::sortition::Config;
use consensus::util::pubkey::PublicKey;

use hex::FromHex;





#[test]
fn test_deterministic_sortition_1() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    // Execute sortition with specific config
    let cfg = Config::new([0; 32], 1, 1, 64);
    p.update_eligibility_flag(cfg.round);

    assert_eq!(
        vec![1, 1, 1, 1, 1],
        Committee::new(PublicKey::default(), &mut p, cfg).get_occurrences()
    );
}

#[test]
fn test_deterministic_sortition_2() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let cfg = Config::new(
        <[u8; 32]>::from_hex("b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5")
            .unwrap_or([0; 32]),
        7777,
        8,
        45,
    );
    p.update_eligibility_flag(cfg.round);

    assert_eq!(
        vec![1, 1, 1, 1, 1],
        Committee::new(PublicKey::default(), &mut p, cfg).get_occurrences()
    );
}

#[test]
fn test_quorum() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let cfg = Config::new(
        <[u8; 32]>::from_hex("b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5")
            .unwrap_or([0; 32]),
        7777,
        8,
        64,
    );
    p.update_eligibility_flag(cfg.round);

    let c = Committee::new(PublicKey::default(), &mut p, cfg);
    assert_eq!(c.quorum(), 4);
}

#[test]
fn test_quorum_max_size() {
    // Create provisioners with bls keys read from an external file.
    let mut p = generate_provisioners(5);

    let cfg = Config::new(
        <[u8; 32]>::from_hex("b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5")
            .unwrap_or([0; 32]),
        7777,
        8,
        4,
    );
    p.update_eligibility_flag(cfg.round);

    let c = Committee::new(PublicKey::default(), &mut p, cfg);
    assert_eq!(c.quorum(), 3);
}

fn generate_provisioners(n: usize) -> Provisioners {
    let mut p = Provisioners::new();
    for i in 0..n {
        let stake_value = 1000 * (i as u64) * DUSK;
        p.add_member_with_value(PublicKey::from_sk_seed_u64(n as u64), stake_value);
    }
    p
}
