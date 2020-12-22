// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(test)]
#![cfg(feature = "host")]

mod external;

use bid_circuits::CorrectnessCircuit;
use bid_contract::BidLeaf;
use bid_contract::Contract;
use canonical_host::{MemStore, Remote, Wasm};
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR, GENERATOR_EXTENDED, GENERATOR_NUMS,
    GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::prelude::*;
use external::RuskExternals;
use poseidon252::{cipher::PoseidonCipher, sponge::sponge::*};

const BYTECODE: &'static [u8] = include_bytes!(
    "../target/wasm32-unknown-unknown/release/bid_contract.wasm"
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

    let pub_params =
        PublicParameters::setup(1 << 11, &mut rand::thread_rng()).unwrap();
    let (pk, vk) = circuit.compile(&pub_params).unwrap();
    circuit.gen_proof(&pub_params, &pk, b"Test").unwrap()
}

#[test]
fn bid_contract_workflow_works() {
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
            &Contract::bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_addr,
                15u64,
                proof.clone(),
                proof,
                1,
                [PublicInput::AffinePoint(commitment, 0, 0).to_bytes()],
            ),
            store.clone(),
            RuskExternals { mem: None },
        )
        .unwrap();
    // If call succeeds, this should not fail.
    cast.commit().unwrap();
    assert!(err == false);
    assert!(idx == 0u64);
}
