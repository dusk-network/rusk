// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

mod mock;

use dusk_jubjub::JubJubScalar;
use mock::{TestNodeClient, TestStore};

use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::{BlsScalar, Proof};
use dusk_poseidon::tree::PoseidonBranch;
use dusk_wallet_core::{
    NodeClient, Store, Transaction, UnprovenTransaction, Wallet, POSEIDON_DEPTH,
};
use phoenix_core::{Note, NoteType};
use rand_core::{CryptoRng, RngCore};

#[derive(Debug)]
struct SerdeNodeClient {
    node: TestNodeClient,
}

impl NodeClient for SerdeNodeClient {
    type Error = ();

    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        self.node.fetch_notes(height, vk)
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        self.node.fetch_anchor()
    }

    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_DEPTH>, Self::Error> {
        self.node.fetch_opening(note)
    }

    fn request_proof(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error> {
        let utx_bytes = utx.to_bytes().expect("Successful serialization");
        let utx_clone = UnprovenTransaction::from_slice(&utx_bytes)
            .expect("Successful deserialization");

        for (input, cinput) in
            utx.inputs().iter().zip(utx_clone.inputs().iter())
        {
            assert_eq!(input.nullifier(), cinput.nullifier());
            // assert_eq!(input.opening(), cinput.opening());
            assert_eq!(input.note(), cinput.note());
            assert_eq!(input.value(), cinput.value());
            assert_eq!(input.blinding_factor(), cinput.blinding_factor());
            assert_eq!(input.pk_r_prime(), cinput.pk_r_prime());
            // assert_eq!(input.signature(), cinput.signature());
        }

        for (output, coutput) in
            utx.outputs().iter().zip(utx_clone.outputs().iter())
        {
            assert_eq!(output, coutput);
        }

        assert_eq!(utx.anchor(), utx_clone.anchor());
        assert_eq!(utx.fee(), utx_clone.fee());
        assert_eq!(utx.crossover(), utx_clone.crossover());
        assert_eq!(utx.call(), utx_clone.call());

        self.node.request_proof(utx)
    }
}

/// Returns obfuscated notes with the given value.
fn new_notes<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    psk: &PublicSpendKey,
    note_values: &[u64],
) -> Vec<Note> {
    note_values
        .iter()
        .map(|val| {
            let blinder = JubJubScalar::random(rng);
            Note::new(rng, NoteType::Obfuscated, psk, *val, blinder)
        })
        .collect()
}

fn new_opening() -> PoseidonBranch<POSEIDON_DEPTH> {
    PoseidonBranch::default()
}

fn new_anchor<Rng: RngCore + CryptoRng>(rng: &mut Rng) -> BlsScalar {
    BlsScalar::random(rng)
}

#[test]
fn serde() {
    let mut rng = rand::thread_rng();

    let send_store = TestStore::new(&mut rng);
    let recv_store = TestStore::new(&mut rng);

    let send_ssk = send_store
        .retrieve_key(0)
        .expect("Valid key when retrieved");
    let recv_ssk = recv_store
        .retrieve_key(0)
        .expect("Valid key when retrieved");

    let send_psk = send_ssk.public_spend_key();
    let recv_psk = recv_ssk.public_spend_key();

    let notes = new_notes(&mut rng, &send_psk, &[2500, 2500, 5000]);
    let anchor = new_anchor(&mut rng);
    let opening = new_opening();

    let node = TestNodeClient::new(notes, anchor, opening);
    let send_wallet =
        Wallet::new(send_store, SerdeNodeClient { node: node.clone() });

    let ref_id = BlsScalar::random(&mut rng);
    let tx = send_wallet
        .create_transfer_tx(
            &mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id,
        )
        .expect("Transaction creation to be successful");

    let tx_bytes = tx.to_bytes().expect("Successful serialization");
    let tx_clone =
        Transaction::from_slice(&tx_bytes).expect("Successful deserialization");

    for (null, cnull) in tx.inputs().iter().zip(tx_clone.inputs().iter()) {
        assert_eq!(null, cnull);
    }

    for (output, coutput) in tx.outputs().iter().zip(tx_clone.outputs().iter())
    {
        assert_eq!(output, coutput);
    }

    assert_eq!(tx.anchor(), tx_clone.anchor());
    assert_eq!(tx.proof(), tx_clone.proof());
    assert_eq!(tx.fee(), tx_clone.fee());
    assert_eq!(tx.crossover(), tx_clone.crossover());
    assert_eq!(tx.call(), tx_clone.call());
}

#[test]
fn create_transfer_tx() {
    let mut rng = rand::thread_rng();

    let send_store = TestStore::new(&mut rng);
    let recv_store = TestStore::new(&mut rng);

    let send_ssk = send_store
        .retrieve_key(0)
        .expect("Valid key when retrieved");
    let recv_ssk = recv_store
        .retrieve_key(0)
        .expect("Valid key when retrieved");

    let send_psk = send_ssk.public_spend_key();
    let recv_psk = recv_ssk.public_spend_key();

    let notes = new_notes(&mut rng, &send_psk, &[2500, 2500, 5000]);
    let anchor = new_anchor(&mut rng);
    let opening = new_opening();

    let node = TestNodeClient::new(notes, anchor, opening);
    let send_wallet = Wallet::new(send_store, node.clone());

    let ref_id = BlsScalar::random(&mut rng);
    let tx = send_wallet
        .create_transfer_tx(
            &mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id,
        )
        .expect("Transaction creation to be successful");

    assert_eq!(tx.inputs().len(), 1);
}

#[test]
fn get_balance() {
    let mut rng = rand::thread_rng();

    let store = TestStore::new(&mut rng);
    let ssk = store.retrieve_key(0).expect("Valid key when retrieved");
    let psk = ssk.public_spend_key();

    let notes = new_notes(&mut rng, &psk, &[2500, 2500, 5000]);
    let anchor = new_anchor(&mut rng);
    let opening = new_opening();

    let node = TestNodeClient::new(notes, anchor, opening);
    let wallet = Wallet::new(store, node.clone());

    let balance = wallet.get_balance(0).expect("Valid balance call");

    assert_eq!(balance, 10000);
}
