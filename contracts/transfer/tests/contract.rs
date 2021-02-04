// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/*
use transfer_circuits::SendToContractTransparentCircuit;
use transfer_contract::Contract;

use canonical_host::{MemStore, Remote, Wasm};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubScalar, GENERATOR_EXTENDED};
use dusk_pki::{Ownable, SecretSpendKey};
use dusk_plonk::circuit_builder::Circuit;
use dusk_plonk::proof_system::proof::Proof;
use external::RuskExternals;
use phoenix_core::Note;
use poseidon252::sponge;
use rand::rngs::StdRng;
use rand::SeedableRng;
use schnorr::single_key::SecretKey as SchnorrSecret;

use std::convert::TryInto;

mod external;

const BYTECODE: &'static [u8] = include_bytes!(
    "../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

#[test]
fn send_to_contract_transparent() {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let store = MemStore::new();
    let contract = Contract::default();
    let wasm = Wasm::new(contract, BYTECODE);
    let mut remote = Remote::new(wasm, &store).unwrap();

    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();

    let address = BlsScalar::random(&mut rng);

    let qr = Contract::<MemStore>::get_balance(address);
    let initial_balance = cast
        .query(&qr, store.clone(), RuskExternals::default())
        .unwrap();

    assert_eq!(0, initial_balance);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = ssk.public_key();

    let value = 100;
    let blinding_factor = JubJubScalar::random(&mut rng);

    let note = Note::obfuscated(&mut rng, &psk, value, blinding_factor);
    let sk_r = ssk.sk_r(note.stealth_address());
    let pk_r = GENERATOR_EXTENDED * sk_r;

    let (_, crossover) = note.try_into().unwrap();
    let value_commitment = *crossover.value_commitment();

    let schnorr_secret = SchnorrSecret::from(sk_r);
    let commitment_hash = sponge::hash(&value_commitment.to_hash_inputs());
    let signature = schnorr_secret.sign(&mut rng, commitment_hash);

    let mut circuit = SendToContractTransparentCircuit::new(
        value_commitment,
        pk_r,
        value,
        blinding_factor,
        signature,
    );

    // Verifier key from Rusk Profile is corrupted
    // https://github.com/dusk-network/rusk/issues/159
    let (pp, pk, _) = circuit.rusk_circuit_args().unwrap();
    let (_, vk) = circuit.compile(&pp).unwrap();

    let proof = circuit.gen_proof(&pp, &pk, b"send-transparent").unwrap();
    let pi = circuit.get_pi_positions().clone();

    let tx = Contract::<MemStore>::send_to_contract_transparent(
        address, value, proof,
    );

    let response = cast
        .transact(&tx, store.clone(), RuskExternals::default())
        .unwrap();
    assert!(response);

    cast.commit().unwrap();

    let qr = Contract::<MemStore>::get_balance(address);
    let balance = cast
        .query(&qr, store.clone(), RuskExternals::default())
        .unwrap();

    assert_eq!(initial_balance + value, balance);
}
*/
