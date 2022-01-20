// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wallet library tests.

mod mock;

use dusk_jubjub::{JubJubAffine, JubJubScalar};
use mock::{mock_wallet, TestProverClient};

use dusk_plonk::prelude::{BlsScalar, Proof};
use dusk_schnorr::Signature;
use dusk_wallet_core::{ProverClient, UnprovenTransaction};
use phoenix_core::{Crossover, Fee};

#[derive(Debug)]
struct SerdeProverClient {
    prover: TestProverClient,
}

impl ProverClient for SerdeProverClient {
    type Error = ();

    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<(), Self::Error> {
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

        self.prover.compute_proof_and_propagate(utx)
    }

    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        self.prover.request_stct_proof(
            fee, crossover, value, blinder, address, signature,
        )
    }

    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        self.prover.request_wfct_proof(commitment, value, blinder)
    }
}

#[test]
fn serde() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}

#[test]
fn transfer() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}

#[test]
fn get_balance() {
    let mut rng = rand::thread_rng();

    let wallet = mock_wallet(&mut rng, &[2500, 2500, 5000]);
    let balance = wallet.get_balance(0).expect("Valid balance call");

    assert_eq!(balance, 10000);
}
