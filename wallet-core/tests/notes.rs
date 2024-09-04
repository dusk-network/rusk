// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ff::Field;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use execution_core::{
    transfer::phoenix::{
        Note, NoteLeaf, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey,
    },
    JubJubScalar,
};

use wallet_core::{
    keys::derive_multiple_phoenix_sk, map_owned, phoenix_balance, BalanceInfo,
};

#[test]
fn test_map_owned() {
    // Assuming this set of notes where the number used as suffix is the
    // "owner":
    // notes := [A1, B1, C2, D2, E1, F3]

    let mut rng = StdRng::seed_from_u64(0xdab);
    const SEED_1: [u8; 64] = [1; 64];
    const SEED_2: [u8; 64] = [2; 64];

    let owner_1_sks = derive_multiple_phoenix_sk(&SEED_1, 0..3);
    let owner_1_pks = [
        PhoenixPublicKey::from(&owner_1_sks[0]),
        PhoenixPublicKey::from(&owner_1_sks[1]),
        PhoenixPublicKey::from(&owner_1_sks[2]),
    ];
    let owner_2_sks = derive_multiple_phoenix_sk(&SEED_2, 0..2);
    let owner_2_pks = [
        PhoenixPublicKey::from(&owner_2_sks[0]),
        PhoenixPublicKey::from(&owner_2_sks[1]),
    ];
    let owner_3_pk =
        PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

    let value = 42;
    let note_leaves = vec![
        gen_note(&mut rng, true, &owner_1_pks[0], value), // owner 1
        gen_note(&mut rng, true, &owner_1_pks[1], value), // owner 1
        gen_note(&mut rng, true, &owner_2_pks[0], value), // owner 2
        gen_note(&mut rng, true, &owner_2_pks[1], value), // owner 2
        gen_note(&mut rng, true, &owner_1_pks[2], value), // owner 1
        gen_note(&mut rng, true, &owner_3_pk, value),     // owner 3
    ];

    // notes with idx 0, 1 and 4 are owned by owner_1
    let notes_by_1 = map_owned(&owner_1_sks, &note_leaves);
    assert_eq!(notes_by_1.len(), 3);
    let note = &note_leaves[0].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[0]);
    assert_eq!(&notes_by_1[&nullifier].note, note);
    let note = &note_leaves[1].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[1]);
    assert_eq!(&notes_by_1[&nullifier].note, note);
    let note = &note_leaves[4].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[2]);
    assert_eq!(&notes_by_1[&nullifier].note, note);

    // notes with idx 2 and 3 are owned by owner_2
    let notes_by_2 = map_owned(&owner_2_sks, &note_leaves);
    assert_eq!(notes_by_2.len(), 2);
    let note = &note_leaves[2].note;
    let nullifier = note.gen_nullifier(&owner_2_sks[0]);
    assert_eq!(&notes_by_2[&nullifier].note, note);
    let note = &note_leaves[3].note;
    let nullifier = note.gen_nullifier(&owner_2_sks[1]);
    assert_eq!(&notes_by_2[&nullifier].note, note);
}

#[test]
fn test_balance() {
    let mut rng = StdRng::seed_from_u64(0xdab);

    let owner_sk = PhoenixSecretKey::random(&mut rng);
    let owner_pk = PhoenixPublicKey::from(&owner_sk);

    let mut notes = Vec::new();

    // create the notes
    for value in 0..=10 {
        // we want to test with a mix of transparent and obfuscated notes so we
        // make every 10th note transparent
        let obfuscated_note = value % 10 != 0;

        notes.push(gen_note(&mut rng, obfuscated_note, &owner_pk, value));

        // also push some notes that are not owned
        if value % 4 == 0 {
            let not_owner_pk =
                PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
            notes.push(gen_note(
                &mut rng,
                obfuscated_note,
                &not_owner_pk,
                value,
            ));
        }
    }

    // the sum of these notes should be 5 * 11 = 55
    // and the spendable notes are 7 + 8 + 9 + 10 = 34
    let expected_balance = BalanceInfo {
        value: 55,
        spendable: 34,
    };

    assert_eq!(
        phoenix_balance(&(&owner_sk).into(), notes.iter()),
        expected_balance
    );
}

fn gen_note(
    rng: &mut StdRng,
    obfuscated_note: bool,
    owner_pk: &PhoenixPublicKey,
    value: u64,
) -> NoteLeaf {
    let sender_pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(rng));

    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    if obfuscated_note {
        NoteLeaf {
            note: Note::obfuscated(
                rng,
                &sender_pk,
                &owner_pk,
                value,
                value_blinder,
                sender_blinder,
            ),
            block_height: rng.gen(),
        }
    } else {
        NoteLeaf {
            note: Note::transparent(
                rng,
                &sender_pk,
                &owner_pk,
                value,
                sender_blinder,
            ),
            block_height: rng.gen(),
        }
    }
}
