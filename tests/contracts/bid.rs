// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bid_circuits::CorrectnessCircuit;
use bid_contract::Contract;
use canonical_host::{MemStore, Remote, Wasm};
use dusk_blindbid::bid::Bid;
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR, GENERATOR_EXTENDED, GENERATOR_NUMS,
    GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use phoenix_core::Note;
use poseidon252::{cipher::PoseidonCipher, sponge::sponge::*};
use rusk::{RuskExtenalError, RuskExternals};
use schnorr::single_key::{PublicKey, SecretKey, Signature};

const BYTECODE: &'static [u8] = include_bytes!(
    "../../contracts/bid/target/wasm32-unknown-unknown/release/bid_contract.wasm"
);
const BID_PROVER_KEY_BYTES: &'static [u8] = include_bytes!(
    "c0e0efc4fc56af4904d52e381eaf5c7090e91e217bc390997a119140dc672ff2.pk"
);

fn create_proof(
    commitment: JubJubAffine,
    value: JubJubScalar,
    blinder: JubJubScalar,
) -> Proof {
    let c = JubJubAffine::from(
        (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
    );

    let mut circuit = CorrectnessCircuit {
        commitment: c,
        value: value.into(),
        blinder: blinder.into(),
        trim_size: 1 << 10,
        pi_positions: vec![],
    };

    let pk = ProverKey::from_bytes(&BID_PROVER_KEY_BYTES)
        .expect("Error generating Bid correctness PK");
    //let (pk, _) = circuit.compile(&pub_params).unwrap();
    circuit
        .gen_proof(&rusk::PUB_PARAMS, &pk, b"BidCorrectness")
        .unwrap()
}

#[test]
fn bid_call_correct_proof_works() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create CorrectnessCircuit Proof and send it
    let value = JubJubScalar::from(100000 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let nonce = BlsScalar::one();
    let encrypted_data = PoseidonCipher::encrypt(
        &[value.into(), blinder.into()],
        &secret,
        &nonce,
    );
    let commitment = JubJubAffine::from(
        &(GENERATOR_EXTENDED * value) + &(GENERATOR_NUMS_EXTENDED * blinder),
    );
    let hashed_secret = sponge_hash(&[value.into()]);
    let pk_r = PublicSpendKey::from(SecretSpendKey::new(value, blinder));
    let stealth_addr = pk_r.gen_stealth_address(&value);
    let proof = create_proof(commitment, value, blinder);
    let mut pub_inp_bytes = [0u8; 33];
    pub_inp_bytes[..].copy_from_slice(
        &PublicInput::AffinePoint(commitment, 0, 0).to_bytes(),
    );
    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, idx) = cast
        .transact(
            &Contract::<MemStore>::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                0u64,
                proof.clone(),
                proof,
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the bid fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(err == false);
    assert!(idx == 0u64);
}

#[test]
fn bid_call_wrong_proof_works() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create CorrectnessCircuit invalid Proof and send it
    let value = JubJubScalar::from(100000 as u64);
    let value_wrong = JubJubScalar::from(100123 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let nonce = BlsScalar::one();
    let encrypted_data = PoseidonCipher::encrypt(
        &[value.into(), blinder.into()],
        &secret,
        &nonce,
    );
    let commitment = JubJubAffine::from(
        &(GENERATOR_EXTENDED * value_wrong)
            + &(GENERATOR_NUMS_EXTENDED * blinder),
    );
    let hashed_secret = sponge_hash(&[value.into()]);
    let pk_r = PublicSpendKey::from(SecretSpendKey::new(value, blinder));
    let stealth_addr = pk_r.gen_stealth_address(&value);
    let proof = create_proof(commitment, value, blinder);
    let mut pub_inp_bytes = [0u8; 33];
    pub_inp_bytes[..].copy_from_slice(
        &PublicInput::AffinePoint(commitment, 0, 0).to_bytes(),
    );
    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, idx) = cast
        .transact(
            &Contract::<MemStore>::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                0u64,
                proof.clone(),
                proof,
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the bid fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(err == true);
}

#[test]
fn extend_bid_correct_sig() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create CorrectnessCircuit invalid Proof and send it
    let value = JubJubScalar::from(100000 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let nonce = BlsScalar::one();
    let encrypted_data = PoseidonCipher::encrypt(
        &[value.into(), blinder.into()],
        &secret,
        &nonce,
    );
    let commitment = JubJubAffine::from(
        &(GENERATOR_EXTENDED * value) + &(GENERATOR_NUMS_EXTENDED * blinder),
    );
    let hashed_secret = sponge_hash(&[value.into()]);
    let secret_spend_key = SecretSpendKey::new(value, blinder);
    let pk_r = PublicSpendKey::from(&secret_spend_key);
    let stealth_addr = pk_r.gen_stealth_address(&value);
    let sk_r = secret_spend_key.sk_r(&stealth_addr);
    let proof = create_proof(commitment, value, blinder);
    let mut pub_inp_bytes = [0u8; 33];
    pub_inp_bytes[..].copy_from_slice(
        &PublicInput::AffinePoint(commitment, 0, 0).to_bytes(),
    );

    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, idx) = cast
        .transact(
            &Contract::<MemStore>::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                0u64,
                proof.clone(),
                proof.clone(),
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the bid fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(err == false);

    // Sign the t_e (expiration) and call extend bid.
    let secret = SecretKey::from(sk_r);
    let signature =
        secret.sign(&mut rand::thread_rng(), BlsScalar::from(10u64));
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let success = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                idx,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(success == true);
}

#[test]
fn extend_bid_wrong_sig() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create CorrectnessCircuit invalid Proof and send it
    let value = JubJubScalar::from(100000 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let nonce = BlsScalar::one();
    let encrypted_data = PoseidonCipher::encrypt(
        &[value.into(), blinder.into()],
        &secret,
        &nonce,
    );
    let commitment = JubJubAffine::from(
        &(GENERATOR_EXTENDED * value) + &(GENERATOR_NUMS_EXTENDED * blinder),
    );
    let hashed_secret = sponge_hash(&[value.into()]);
    let secret_spend_key = SecretSpendKey::new(value, blinder);
    let pk_r = PublicSpendKey::from(&secret_spend_key);
    let stealth_addr = pk_r.gen_stealth_address(&value);
    let sk_r = secret_spend_key.sk_r(&stealth_addr);
    let proof = create_proof(commitment, value, blinder);
    let mut pub_inp_bytes = [0u8; 33];
    pub_inp_bytes[..].copy_from_slice(
        &PublicInput::AffinePoint(commitment, 0, 0).to_bytes(),
    );
    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, idx) = cast
        .transact(
            &Contract::<MemStore>::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                0u64,
                proof.clone(),
                proof.clone(),
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the bid fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(err == false);

    // Sign the t_e (expiration) and call extend bid.
    let secret = SecretKey::from(sk_r);
    let signature =
        secret.sign(&mut rand::thread_rng(), BlsScalar::from(50u64));
    assert!(signature
        .verify(
            &PublicKey::from(stealth_addr.pk_r()),
            BlsScalar::from(50u64)
        )
        .is_ok());
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let success = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                idx,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(success == false);
}

#[test]
fn bid_correct_withdraw() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    // Create CorrectnessCircuit Proof and send it
    let value = JubJubScalar::from(100000 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let nonce = BlsScalar::one();
    let encrypted_data = PoseidonCipher::encrypt(
        &[value.into(), blinder.into()],
        &secret,
        &nonce,
    );
    let commitment = JubJubAffine::from(
        &(GENERATOR_EXTENDED * value) + &(GENERATOR_NUMS_EXTENDED * blinder),
    );
    let hashed_secret = sponge_hash(&[value.into()]);
    let secret_spend_key = SecretSpendKey::new(value, blinder);
    let pk_r = PublicSpendKey::from(&secret_spend_key);
    let stealth_addr = pk_r.gen_stealth_address(&value);
    let sk_r = secret_spend_key.sk_r(&stealth_addr);
    let proof = create_proof(commitment, value, blinder);
    let mut pub_inp_bytes = [0u8; 33];
    pub_inp_bytes[..].copy_from_slice(
        &PublicInput::AffinePoint(commitment, 0, 0).to_bytes(),
    );
    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, idx) = cast
        .transact(
            &Contract::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                0u64,
                proof.clone(),
                proof.clone(),
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .unwrap();
    // If call succeeds, this should not fail.
    cast.commit().unwrap();
    assert!(err == false);
    assert!(idx == 0u64);

    // Sign the t_e (expiration) and call withdraw bid.
    let secret = SecretKey::from(sk_r);

    // Note that the block_height has to be set so that it
    // surpasses t_e after the extension + COOLDOWN_PERIOD.
    let signature =
        secret.sign(&mut rand::thread_rng(), BlsScalar::from(10u64));
    let block_height = 0u64;
    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &pk_r, 55);
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let success = cast
        .transact(
            &Contract::<MemStore>::withdraw(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                note,
                proof.clone(),
                idx,
                block_height,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(success == true);
}
