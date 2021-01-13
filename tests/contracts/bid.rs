// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bid_circuits::CorrectnessCircuit;
use bid_contract::{contract_constants::*, Contract};
use canonical_host::{MemStore, Remote, Wasm};
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use phoenix_core::Note;
use poseidon252::{cipher::PoseidonCipher, sponge::sponge::*};
use rusk::RuskExternals;
use schnorr::single_key::{PublicKey, SecretKey};

const BYTECODE: &'static [u8] = include_bytes!(
    "../../contracts/bid/target/wasm32-unknown-unknown/release/bid_contract.wasm"
);
const BID_PROVER_KEY_BYTES: &'static [u8] = include_bytes!(
    "c0e0efc4fc56af4904d52e381eaf5c7090e91e217bc390997a119140dc672ff2.pk"
);

fn create_proof(value: JubJubScalar, blinder: JubJubScalar) -> Proof {
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
    circuit
        .gen_proof(&rusk::PUB_PARAMS, &pk, b"BidCorrectness")
        .unwrap()
}

fn setup_test_params() -> (
    JubJubScalar,
    JubJubScalar,
    JubJubAffine,
    BlsScalar,
    BlsScalar,
    PoseidonCipher,
    PublicSpendKey,
    SecretSpendKey,
    StealthAddress,
    u64,
) {
    let value = JubJubScalar::from(100000 as u64);
    let blinder = JubJubScalar::from(50000 as u64);
    let secret = JubJubAffine::from(GENERATOR_EXTENDED * value);
    let ssk = SecretSpendKey::new(value, blinder);
    let psk = PublicSpendKey::from(ssk);
    let nonce = BlsScalar::one();
    (
        value,
        blinder,
        JubJubAffine::from(
            &(GENERATOR_EXTENDED * value)
                + &(GENERATOR_NUMS_EXTENDED * blinder),
        ),
        sponge_hash(&[value.into()]),
        BlsScalar::one(),
        PoseidonCipher::encrypt(
            &[value.into(), blinder.into()],
            &secret,
            &nonce,
        ),
        psk,
        ssk,
        psk.gen_stealth_address(&value),
        0u64,
    )
}

#[test]
fn bid_call_correct_proof_works() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();
    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        _,
        _,
        stealth_addr,
        block_height,
    ) = setup_test_params();

    // Create CorrectnessCircuit Proof and send it.
    // The proof in this case is correct but the public inputs aren't.
    let proof = create_proof(value, blinder);

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
                block_height,
                proof.clone(),
                proof,
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
    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        _,
        _,
        stealth_addr,
        block_height,
    ) = setup_test_params();

    // Create CorrectnessCircuit invalid Proof and send it
    let proof = create_proof(value, blinder);
    // Add leaf to the Contract's tree and get it's pos index back
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let (err, _) = cast
        .transact(
            &Contract::<MemStore>::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                block_height,
                proof.clone(),
                proof,
                [PublicInput::AffinePoint(JubJubAffine::identity(), 0, 0)
                    .to_bytes()],
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
fn extend_bid_updates_expiration() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        _,
        ssk,
        stealth_addr,
        block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit invalid Proof and send it
    let proof = create_proof(value, blinder);

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
                block_height,
                proof.clone(),
                proof.clone(),
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

    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    //
    // Sign the t_e (expiration) and call extend bid.
    let secret = SecretKey::from(sk_r);
    let signature = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + EXPIRATION_PERIOD + MATURITY_PERIOD),
    );
    let call_error = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == false);

    // If the latest call updated correctly the expiration time of
    // our Bid. If we want to extend it again, we should sign now the
    // new expiration time. Which is equivalent to expiration +
    // EXPIRATION_PERIOD.
    let signature2 = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + 2 * EXPIRATION_PERIOD + MATURITY_PERIOD),
    );
    let call_error = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature2,
                PublicKey::from(stealth_addr.pk_r()),
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == false);
}

#[test]
fn extend_bid_wrong_sig() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        _,
        ssk,
        stealth_addr,
        block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit invalid Proof and send it
    let proof = create_proof(value, blinder);

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
                block_height,
                proof.clone(),
                proof.clone(),
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

    // Sign the t_e (expiration) and call extend bid.
    let secret = SecretKey::from(sk_r);
    // Use as a message a wrong bid expiration.
    let message = BlsScalar::from(50u64);
    let signature = secret.sign(&mut rand::thread_rng(), message);
    assert!(signature
        .verify(&PublicKey::from(stealth_addr.pk_r()), message)
        .is_ok());
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == true);
}

#[test]
fn extend_bid_with_unrecorded_pub_key() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        _,
        ssk,
        stealth_addr,
        block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit invalid Proof and send it
    let proof = create_proof(value, blinder);

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
                block_height,
                proof.clone(),
                proof.clone(),
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

    // Sign the t_e (expiration) and call extend bid.
    // Note that this does not really matter since the Public
    // key that we will send through the call is not valid.
    // So the signature will never be checked.
    let secret = SecretKey::from(sk_r);
    let message =
        BlsScalar::from(block_height + EXPIRATION_PERIOD + MATURITY_PERIOD);
    let signature = secret.sign(&mut rand::thread_rng(), message);

    // Call the signature method with a `PublicKey` that is not found
    // in any map entry in the contract.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::extend_bid(
                signature,
                PublicKey::from(
                    PublicSpendKey::from(SecretSpendKey::new(
                        JubJubScalar::one(),
                        JubJubScalar::one(),
                    ))
                    .gen_stealth_address(&JubJubScalar::one())
                    .pk_r(),
                ),
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == true);
}

#[test]
fn bid_correct_withdraw() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        psk,
        ssk,
        stealth_addr,
        mut block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit Proof and send it
    let proof = create_proof(value, blinder);
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
                block_height,
                proof.clone(),
                proof.clone(),
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
    let signature = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + EXPIRATION_PERIOD + MATURITY_PERIOD),
    );
    block_height = block_height
        + EXPIRATION_PERIOD
        + MATURITY_PERIOD
        + COOLDOWN_PERIOD
        + 1;
    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &psk, 55);
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::withdraw(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                note,
                proof.clone(),
                block_height,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == false);
}

#[test]
fn bid_withdraw_with_low_block_height() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        psk,
        ssk,
        stealth_addr,
        mut block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit Proof and send it
    let proof = create_proof(value, blinder);
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
                block_height,
                proof.clone(),
                proof.clone(),
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
    // Since this is not done, the call should fail.
    let signature = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + MATURITY_PERIOD + EXPIRATION_PERIOD),
    );
    block_height =
        block_height + EXPIRATION_PERIOD + MATURITY_PERIOD + COOLDOWN_PERIOD
            - 1;
    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &psk, 55);
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    //
    // It will fail since the sent `block_height` is lower than the required by
    // the expiration timestamp of the stored bid.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::withdraw(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                note,
                proof.clone(),
                block_height,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == true);
}

#[test]
fn bid_withdraw_with_wrong_sig() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        psk,
        ssk,
        stealth_addr,
        mut block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit Proof and send it
    let proof = create_proof(value, blinder);
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
                block_height,
                proof.clone(),
                proof.clone(),
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

    // Here we are signing a wrong message so that the call fails.
    let signature = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + EXPIRATION_PERIOD + MATURITY_PERIOD + 1),
    );

    // Note that the block_height has to be set so that it
    // surpasses t_e after the extension + COOLDOWN_PERIOD.
    block_height = block_height
        + EXPIRATION_PERIOD
        + MATURITY_PERIOD
        + COOLDOWN_PERIOD
        + 1;
    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &psk, 55);

    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    // This should fail since the signature is wrong.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::withdraw(
                signature,
                PublicKey::from(stealth_addr.pk_r()),
                note,
                proof.clone(),
                block_height,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == true);
}

#[test]
fn bid_withdraw_with_unrecorded_pub_key() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let (
        value,
        blinder,
        commitment,
        hashed_secret,
        nonce,
        encrypted_data,
        psk,
        ssk,
        stealth_addr,
        mut block_height,
    ) = setup_test_params();
    let sk_r = ssk.sk_r(&stealth_addr);

    // Create CorrectnessCircuit Proof and send it
    let proof = create_proof(value, blinder);
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
                block_height,
                proof.clone(),
                proof.clone(),
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
    let signature = secret.sign(
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + EXPIRATION_PERIOD + MATURITY_PERIOD),
    );

    // Note that the block_height has to be set so that it
    // surpasses t_e after the extension + COOLDOWN_PERIOD.
    block_height = block_height
        + MATURITY_PERIOD
        + EXPIRATION_PERIOD
        + COOLDOWN_PERIOD
        + 1;
    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &psk, 55);

    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    // This should fail since the `PublicKey` we're sending does not correspond
    // to any entries in the contract's map.
    let call_error = cast
        .transact(
            &Contract::<MemStore>::withdraw(
                signature,
                PublicKey::from(
                    PublicSpendKey::from(SecretSpendKey::new(
                        JubJubScalar::one(),
                        JubJubScalar::one(),
                    ))
                    .gen_stealth_address(&JubJubScalar::one())
                    .pk_r(),
                ),
                note,
                proof.clone(),
                block_height,
            ),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call extend_bid method");

    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
    assert!(call_error == true);
}
