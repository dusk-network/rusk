// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::net;

use bid_circuits::BidCorrectnessCircuit;
use bid_contract::{contract_constants::*, Contract as BidContract};
use dusk_blindbid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{PublicKey, PublicSpendKey, SecretKey, SecretSpendKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::{cipher::PoseidonCipher, sponge};
use dusk_schnorr::{PublicKeyPair, Signature};
use lazy_static::lazy_static;
use phoenix_core::{Message, Note};
use rusk_abi::RuskModule;
use rusk_vm::{Contract, ContractId, GasMeter, NetworkState};

const BYTECODE: &'static [u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/bid_contract.wasm"
);

lazy_static! {
    pub(crate) static ref PUB_PARAMS: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

fn create_proof(value: JubJubScalar, blinder: JubJubScalar) -> Proof {
    let c = JubJubAffine::from(
        (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
    );

    let mut circuit = BidCorrectnessCircuit {
        commitment: c,
        value: value.into(),
        blinder: blinder.into(),
    };

    let pk = rusk_profile::keys_for(&BidCorrectnessCircuit::CIRCUIT_ID)
        .expect("Failed to fetch circuit keys")
        .get_prover()
        .expect("Failed to get proverkey data");
    let pk = ProverKey::from_slice(&pk)
        .expect("Failed to deserialize the ProverKey");
    circuit.gen_proof(&PUB_PARAMS, &pk, b"Test").unwrap()
}

#[test]
fn bid_contract_workflow_works() {
    // Init Env & Contract
    let contract = Contract::new(BidContract::new(), BYTECODE.to_vec());
    // Create BidCorrectnessCircuit Proof and send it
    let (a, b) = (
        JubJubScalar::random(&mut rand::thread_rng()),
        JubJubScalar::random(&mut rand::thread_rng()),
    );
    let secret = JubJubScalar::random(&mut rand::thread_rng());
    let hashed_secret = sponge::hash(&[secret.into()]);
    let secret_spend_key = SecretSpendKey::new(a, b);
    let psk = PublicSpendKey::from(&secret_spend_key);
    let stealth_addr = psk.gen_stealth_address(&a);
    let sk_r = secret_spend_key.sk_r(&stealth_addr);
    let sk = SecretKey::from(sk_r);
    let pk = PublicKey::from(&sk);
    let proof = create_proof(a, b);
    let message = Message::new(&mut rand::thread_rng(), &secret, &psk, 25u64);

    // Generate env
    let mut block_height = 0u64;
    let mut network = NetworkState::with_block_height(block_height);
    let rusk_mod = RuskModule::new(&*PUB_PARAMS);
    network.register_host_module(rusk_mod);
    // Deploy contract
    let contract_id = network.deploy(contract).expect("Deploy failure");
    let mut gas = GasMeter::with_limit(1_000_000_000);

    // Add leaf to the Contract's tree and get it's pos index back
    let call_result = network
        .transact::<_, bool>(
            contract_id,
            (
                bid_contract::ops::BID,
                message,
                hashed_secret,
                stealth_addr,
                proof.to_bytes().to_vec(),
                proof.to_bytes().to_vec(),
            ),
            &mut gas,
        )
        .expect("Bid Transaction error");

    assert!(call_result);

    // Set a valid block height so that the Bid is withdrawable.
    // TODO

    // Sign the t_e (expiration) and call extend bid.
    let signature = Signature::new(
        &sk,
        &mut rand::thread_rng(),
        BlsScalar::from(VALIDITY_PERIOD),
    );

    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let call_result = network
        .transact::<_, bool>(
            contract_id,
            (bid_contract::ops::EXTEND_BID, signature, pk),
            &mut gas,
        )
        .expect("Failed to call extend_bid method");

    assert!(call_result);

    // Sign the t_e (expiration) and call withdraw bid..
    let signature = Signature::new(
        &sk,
        &mut rand::thread_rng(),
        BlsScalar::from(block_height),
    );

    // Set a valid block height so that the Bid is withdrawable.
    // TODO

    // Create a Note
    // TODO: Create a correct note once the inter-contract call is implemented.
    let note = Note::obfuscated(&mut rand::thread_rng(), &psk, 55, b);
    // Now that a Bid is inside the tree we should be able to extend it if the
    // correct signature is provided.
    let call_result = network
        .transact::<_, bool>(
            contract_id,
            (
                bid_contract::ops::WITHDRAW,
                signature,
                pk,
                note,
                proof.clone().to_bytes().to_vec(),
            ),
            &mut gas,
        )
        .expect("Failed to call extend_bid method");

    assert!(call_result);
}
