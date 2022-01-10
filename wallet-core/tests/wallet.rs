// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

use dusk_pki::ViewKey;
use dusk_plonk::prelude::{BlsScalar, Proof};
use dusk_poseidon::tree::PoseidonBranch;
use dusk_wallet_core::test_utils::mock_wallet;
use dusk_wallet_core::{
    test_utils::TestNodeClient, NodeClient, Transaction, UnprovenTransaction,
    POSEIDON_DEPTH,
};
use phoenix_core::Note;

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

#[test]
fn serde() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    let tx = wallet
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

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    let tx = wallet
        .create_transfer_tx(
            &mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id,
        )
        .expect("Transaction creation to be successful");

    assert_eq!(tx.inputs().len(), 1);
}

#[test]
fn get_balance() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);
    let balance = wallet.get_balance(0).expect("Valid balance call");

    assert_eq!(balance, 10000);
}
