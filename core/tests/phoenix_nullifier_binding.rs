// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_core::transfer::Transaction;
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::phoenix::{
    Note, NoteTreeItem, NotesTree, Payload, Prove,
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey, TxCircuitVec,
    TxSkeleton,
};
use dusk_core::{BlsScalar, Error, JubJubScalar};
use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED, JubJubAffine};
use dusk_poseidon::{Domain, Hash};
use ff::Field;
use jubjub_schnorr::{Error as SchnorrError, PublicKeyDouble, SignatureDouble};
use rand::SeedableRng;
use rand::rngs::StdRng;

const CHAIN_ID: u8 = 0xFA;
const NOTE_VALUE: u64 = 42;
const GAS_LIMIT: u64 = 1;
const GAS_PRICE: u64 = 1;

struct NoProver;

impl Prove for NoProver {
    fn prove(&self, tx_circuit_vec_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(TxCircuitVec::from_slice(tx_circuit_vec_bytes)
            .expect("serialization should be ok")
            .to_var_bytes()
            .to_vec())
    }
}

#[test]
fn equivocated_note_pk_prime_changes_signed_payload_hash() {
    let mut rng = StdRng::seed_from_u64(0x51ade);

    let sender_sk = PhoenixSecretKey::random(&mut rng);
    let sender_pk = PhoenixPublicKey::from(&sender_sk);
    let receiver_pk =
        PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

    let mut notes_tree = NotesTree::new();
    let value_blinder = JubJubScalar::random(&mut rng);
    let sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    let mut note = Note::obfuscated(
        &mut rng,
        &sender_pk,
        &sender_pk,
        NOTE_VALUE,
        value_blinder,
        sender_blinder,
    );
    note.set_pos(0);
    notes_tree.insert(
        *note.pos(),
        NoteTreeItem {
            hash: note.hash(),
            data: (),
        },
    );
    let opening = notes_tree
        .opening(*note.pos())
        .expect("there should be a note at the given position");
    let root = notes_tree.root().hash;

    let transfer_value = NOTE_VALUE - GAS_LIMIT * GAS_PRICE;
    let tx = Transaction::phoenix(
        &mut rng,
        &sender_sk,
        &sender_pk,
        &receiver_pk,
        vec![(note, opening)],
        root,
        transfer_value,
        false,
        0,
        GAS_LIMIT,
        GAS_PRICE,
        CHAIN_ID,
        Option::<TransactionData>::None,
        &NoProver,
    )
    .expect("phoenix transaction should be created");

    let Transaction::Phoenix(tx) = tx else {
        panic!("expected phoenix transaction")
    };
    let circuit = TxCircuitVec::from_slice(tx.proof())
        .expect("NoProver stores circuit bytes in the proof field");
    let input = &circuit.input_notes_info[0];
    let payload_hash = tx.payload_hash();
    let note_sk = sender_sk.gen_note_sk(input.note.stealth_address());
    let note_pk = *input.note.stealth_address().note_pk().as_ref();

    let honest_pk =
        PublicKeyDouble::from_raw_unchecked(note_pk, input.note_pk_p.into());
    honest_pk
        .verify(&input.signature, payload_hash)
        .expect("honest input signature should verify");

    let r = JubJubScalar::random(&mut rng);
    let r_point = GENERATOR_EXTENDED * r;
    let r_p_point = GENERATOR_NUMS_EXTENDED * JubJubScalar::random(&mut rng);

    let r_coords = r_point.to_hash_inputs();
    let r_p_coords = r_p_point.to_hash_inputs();
    let pk_coords = note_pk.to_hash_inputs();
    // This mirrors the pinned jubjub-schnorr 0.7.0-rc.0 double-signature
    // challenge over R, R', pk, msg. That scheme omits pk_prime, which is
    // what lets a key holder equivocate note_pk_p for a fixed payload hash.
    let c = Hash::digest_truncated(
        Domain::Other,
        &[
            r_coords[0],
            r_coords[1],
            r_p_coords[0],
            r_p_coords[1],
            pk_coords[0],
            pk_coords[1],
            payload_hash,
        ],
    )[0];

    let u = r - (c * note_sk.as_ref());
    let equivocated_note_pk_p =
        (r_p_point - GENERATOR_NUMS_EXTENDED * u) * c.invert().unwrap();
    assert_ne!(equivocated_note_pk_p, input.note_pk_p.into());

    let r_affine: JubJubAffine = r_point.into();
    let r_p_affine: JubJubAffine = r_p_point.into();
    let mut signature_bytes = [0u8; SignatureDouble::SIZE];
    signature_bytes[..32].copy_from_slice(&u.to_bytes());
    signature_bytes[32..64].copy_from_slice(&r_affine.to_bytes());
    signature_bytes[64..].copy_from_slice(&r_p_affine.to_bytes());
    let equivocated_signature =
        SignatureDouble::from_bytes(&signature_bytes).unwrap();
    let equivocated_pk =
        PublicKeyDouble::from_raw_unchecked(note_pk, equivocated_note_pk_p);

    equivocated_pk
        .verify(&equivocated_signature, payload_hash)
        .expect("old fixed-message key-holder equivocation should verify");

    let equivocated_coords = equivocated_note_pk_p.to_hash_inputs();
    let equivocated_nullifier = Hash::digest(
        Domain::Other,
        &[
            equivocated_coords[0],
            equivocated_coords[1],
            BlsScalar::from(*input.note.pos()),
        ],
    )[0];
    assert_ne!(equivocated_nullifier, input.nullifier);

    let equivocated_payload = Payload {
        chain_id: CHAIN_ID,
        tx_skeleton: TxSkeleton {
            root,
            nullifiers: vec![equivocated_nullifier],
            outputs: tx.outputs().clone(),
            max_fee: tx.fee().max_fee(),
            deposit: tx.deposit(),
        },
        fee: *tx.fee(),
        data: None,
    };
    let equivocated_payload_hash = equivocated_payload.hash();
    assert_ne!(equivocated_payload_hash, payload_hash);

    assert_eq!(
        equivocated_pk
            .verify(&equivocated_signature, equivocated_payload_hash)
            .unwrap_err(),
        SchnorrError::InvalidSignature,
    );
}
