// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ff::Field;
use rand::{rngs::StdRng, CryptoRng, RngCore, SeedableRng};

use execution_core::{
    transfer::phoenix::{
        Note, NoteLeaf, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
    },
    JubJubScalar,
};

use wallet_core::{
    keys::derive_multiple_phoenix_sk, keys::derive_phoenix_sk, map_owned,
    notes::owned::NoteList, phoenix_balance, pick_notes, BalanceInfo, Seed,
};

/// Generate a note, useful for testing purposes
fn gen_note<T: RngCore + CryptoRng>(
    rng: &mut T,
    obfuscated_note: bool,
    owner_pk: &PhoenixPublicKey,
    value: u64,
) -> Note {
    let sender_pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(rng));

    let value_blinder = JubJubScalar::random(&mut *rng);
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    if obfuscated_note {
        Note::obfuscated(
            rng,
            &sender_pk,
            owner_pk,
            value,
            value_blinder,
            sender_blinder,
        )
    } else {
        Note::transparent(rng, &sender_pk, owner_pk, value, sender_blinder)
    }
}

/// Generate a note leaf, useful for testing purposes
fn gen_note_leaf<T: RngCore + CryptoRng>(
    rng: &mut T,
    obfuscated_note: bool,
    owner_pk: &PhoenixPublicKey,
    value: u64,
) -> NoteLeaf {
    let block_height = 1;
    let note = gen_note(rng, obfuscated_note, owner_pk, value);

    NoteLeaf { note, block_height }
}

#[test]
fn test_map_owned() {
    let mut rng = StdRng::seed_from_u64(0xdab);
    const SEED: Seed = [1; 64];

    let owner_1_sks = derive_multiple_phoenix_sk(&SEED, 0..3);
    let owner_1_pks = [
        PhoenixPublicKey::from(&owner_1_sks[0]),
        PhoenixPublicKey::from(&owner_1_sks[1]),
        PhoenixPublicKey::from(&owner_1_sks[2]),
    ];
    let owner_2_sks = derive_multiple_phoenix_sk(&SEED, 3..5);
    let owner_2_pks = [
        PhoenixPublicKey::from(&owner_2_sks[0]),
        PhoenixPublicKey::from(&owner_2_sks[1]),
    ];
    let owner_3_pk = PhoenixPublicKey::from(&derive_phoenix_sk(&SEED, 5));

    let note_leaves = vec![
        gen_note(&mut rng, true, &owner_1_pks[0], 12), // owner 1
        gen_note(&mut rng, true, &owner_1_pks[1], 1),  // owner 1
        gen_note(&mut rng, true, &owner_2_pks[0], 3),  // owner 2
        gen_note(&mut rng, true, &owner_2_pks[1], 76), // owner 2
        gen_note(&mut rng, true, &owner_1_pks[2], 6),  // owner 1
        gen_note(&mut rng, true, &owner_3_pk, 42),     // owner 3
    ];

    let note_leaves: Vec<NoteLeaf> = note_leaves
        .into_iter()
        .map(|note| NoteLeaf {
            note,
            block_height: 0,
        })
        .collect();

    // notes with idx 0, 1 and 4 are owned by owner_1
    let notes_by_1 = map_owned(&owner_1_sks, &note_leaves);
    assert_eq!(notes_by_1.len(), 3);

    let note = &note_leaves[0].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[0]);
    assert_eq!(&notes_by_1[0][&nullifier].note, note);
    let note = &note_leaves[1].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[1]);
    assert_eq!(&notes_by_1[1][&nullifier].note, note);
    let note = &note_leaves[4].note;
    let nullifier = note.gen_nullifier(&owner_1_sks[2]);
    assert_eq!(&notes_by_1[2][&nullifier].note, note);

    // notes with idx 2 and 3 are owned by owner_2
    let notes_by_2 = map_owned(&owner_2_sks, &note_leaves);
    assert_eq!(notes_by_2.len(), 2);
    let note = &note_leaves[2].note;
    let nullifier = note.gen_nullifier(&owner_2_sks[0]);
    assert_eq!(&notes_by_2[0][&nullifier].note, note);
    let note = &note_leaves[3].note;
    let nullifier = note.gen_nullifier(&owner_2_sks[1]);
    assert_eq!(&notes_by_2[1][&nullifier].note, note);
}

#[test]
fn test_balance() {
    let mut rng = StdRng::seed_from_u64(0xdab);

    let owner_sk = PhoenixSecretKey::random(&mut rng);
    let owner_pk = PhoenixPublicKey::from(&owner_sk);

    let mut notes = Vec::new();

    // create the notes
    for value in 0..=10 {
        // we want to test with a mix of transparent and obfuscated notes so
        // we make every 10th note transparent
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

    let notes: Vec<NoteLeaf> = notes
        .into_iter()
        .map(|note| NoteLeaf {
            note,
            block_height: 0,
        })
        .collect();

    assert_eq!(
        phoenix_balance(&(&owner_sk).into(), notes.iter()),
        expected_balance
    );
}

#[test]
fn test_pick_notes() {
    use rand::SeedableRng;

    let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(0xbeef);

    let sk = PhoenixSecretKey::random(&mut rng);
    let vk = PhoenixViewKey::from(&sk);
    let pk = PhoenixPublicKey::from(&sk);

    // Check single input-note

    // sanity check
    assert!(pick_notes(&vk, NoteList::default(), 70).is_empty());

    // basic check with one note
    let leaf = gen_note_leaf(&mut rng, true, &pk, 100);
    let all_notes = generate_note_list([leaf], &sk);

    let input_notes = pick_notes(&vk, all_notes.clone(), 70);
    assert_eq!(input_notes, all_notes);

    // out of balance basic check
    let leaf = gen_note_leaf(&mut rng, true, &pk, 100);
    let all_notes = generate_note_list([leaf], &sk);
    assert!(pick_notes(&vk, all_notes, 101).is_empty());

    // Check multiple input-notes

    // checking a naive, simple order-based output works
    let leaves = [
        gen_note_leaf(&mut rng, true, &pk, 100),
        gen_note_leaf(&mut rng, true, &pk, 500),
        gen_note_leaf(&mut rng, true, &pk, 300),
    ];
    let all_notes = generate_note_list(leaves, &sk);
    assert_eq!(pick_notes(&vk, all_notes.clone(), 600), all_notes);

    // checking that spending more than the total doesn't work
    let leaves = [
        gen_note_leaf(&mut rng, true, &pk, 100),
        gen_note_leaf(&mut rng, true, &pk, 500),
        gen_note_leaf(&mut rng, true, &pk, 300),
    ];
    let all_notes = generate_note_list(leaves, &sk);
    assert_eq!(pick_notes(&vk, all_notes, 901), NoteList::default());

    // checking that pick_notes works if spendable is smaller that the total

    // generate 5 notes with 10 dusk owned by the same key
    let leaves = [
        gen_note_leaf(&mut rng, true, &pk, 10),
        gen_note_leaf(&mut rng, true, &pk, 10),
        gen_note_leaf(&mut rng, true, &pk, 10),
        gen_note_leaf(&mut rng, true, &pk, 10),
        gen_note_leaf(&mut rng, true, &pk, 20),
    ];
    let all_notes = generate_note_list(leaves, &sk);

    // with a target value of 20 it should pick the first 4 notes
    let target = 20;
    let expected_input_notes =
        generate_expected_input_notes(&all_notes, &[0, 1, 2, 3]);
    assert_eq!(
        pick_notes(&vk, all_notes.clone(), target),
        expected_input_notes
    );

    // with a target value of 50 it should also pick the last note
    let target = 50;
    let expected_input_notes =
        generate_expected_input_notes(&all_notes, &[0, 1, 2, 4]);
    assert_eq!(
        pick_notes(&vk, all_notes.clone(), target),
        expected_input_notes
    );

    // a target value of 51 however is above the max spendable
    let target = 51;
    let expected_input_notes = NoteList::default();
    assert_eq!(
        pick_notes(&vk, all_notes.clone(), target),
        expected_input_notes
    );
}

fn generate_expected_input_notes(
    ordered_notes: &NoteList,
    expected_indices: &[usize],
) -> NoteList {
    let expected_input: Vec<(_, _)> = ordered_notes
        .iter()
        .enumerate()
        .filter_map(|(i, note_leaf)| {
            if expected_indices.contains(&i) {
                Some(note_leaf.clone())
            } else {
                None
            }
        })
        .collect();
    expected_input.into()
}

fn generate_note_list(
    note_leaves: impl AsRef<[NoteLeaf]>,
    sk: &PhoenixSecretKey,
) -> NoteList {
    note_leaves
        .as_ref()
        .iter()
        .map(|leaf| {
            let nullifier = leaf.note.gen_nullifier(sk);
            (nullifier, leaf.clone())
        })
        .collect::<Vec<(_, _)>>()
        .into()
}
