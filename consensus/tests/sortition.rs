// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use consensus::user::committee::Committee;
use consensus::user::provisioners::{Provisioners, PublicKey, DUSK};
use consensus::user::sortition::Config;

use hex::FromHex;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[test]
fn test_deterministic_sortition_1() {
    // Create provisioners with bls keys read from an external file.
    let mut p = read_provisioners();

    // Execute sortition with specific config
    let cfg = Config([0; 32], 1, 1, 64);
    p.update_eligibility_flag(cfg.1);

    assert_eq!(
        vec![4, 13, 9, 16, 22],
        Committee::new(PublicKey::default(), &mut p, cfg).get_occurrences()
    );
}

#[test]
fn test_deterministic_sortition_2() {
    // Create provisioners with bls keys read from an external file.
    let mut p = read_provisioners();

    let cfg = Config(
        <[u8; 32]>::from_hex("b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5")
            .unwrap_or([0; 32]),
        7777,
        8,
        45,
    );
    p.update_eligibility_flag(cfg.1);

    assert_eq!(
        vec![1, 5, 13, 11, 15],
        Committee::new(PublicKey::default(), &mut p, cfg).get_occurrences()
    );
}

fn read_provisioners() -> Provisioners {
    let test_data = env::var("CARGO_MANIFEST_DIR").unwrap_or_default() + "/tests/provisioners.txt";

    // Create provisioners with bls keys read from an external file.
    let mut p = Provisioners::new();
    if let Ok(lines) = read_lines(test_data) {
        let mut i = 1;
        for line in lines {
            if let Ok(bls_key) = line {
                // parse hex from file line
                let key = <[u8; 96]>::from_hex(bls_key).unwrap_or([0; 96]);
                let stake_value = 1000 * i * DUSK;

                p.add_member_with_value(PublicKey::new(key), stake_value);

                i += 1;
            }
        }
    }
    p
}
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
