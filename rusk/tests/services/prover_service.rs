// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TestContext;

use std::sync::Mutex;

use dusk_pki::{PublicKey, PublicSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    ProverClient, StateClient, Store, UnprovenTransaction, Wallet,
    POSEIDON_TREE_DEPTH,
};
use phoenix_core::{Crossover, Fee, Note, NoteType};
use rand::{CryptoRng, RngCore};
use rusk::services::rusk_proto::prover_client::ProverClient as ProverGrpcClient;
use rusk::services::rusk_proto::ExecuteProverRequest;
use test_context::test_context;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tonic::transport::Channel;

/// Create a new wallet meant for tests. It includes a client that will always
/// return a random anchor (same every time), and the default opening.
///
/// The number of notes available is determined by `note_values`.
fn mock_wallet<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    client: ProverGrpcClient<Channel>,
    note_values: &[u64],
) -> Wallet<TestStore, TestStateClient, TestProverClient> {
    let store = TestStore::new(rng);
    let psk = store.retrieve_ssk(0).unwrap().public_spend_key();

    let notes = new_notes(rng, &psk, note_values);
    let anchor = BlsScalar::random(rng);
    let opening = Default::default();

    let state = TestStateClient::new(notes, anchor, opening);
    let prover = TestProverClient::new(client);

    Wallet::new(store, state, prover)
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

/// An in-memory seed store.
#[derive(Debug)]
pub struct TestStore {
    seed: [u8; 64],
}

impl TestStore {
    /// Instantiate a new in-memory store with a random seed.
    fn new<Rng: RngCore + CryptoRng>(rng: &mut Rng) -> Self {
        let mut seed = [0; 64];
        rng.fill_bytes(&mut seed);
        Self { seed }
    }
}

impl Store for TestStore {
    type Error = ();

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }
}

#[derive(Debug)]
struct TestStateClient {
    notes: Vec<Note>,
    anchor: BlsScalar,
    opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
}

#[derive(Debug)]
struct TestProverClient {
    client: Mutex<ProverGrpcClient<Channel>>,
}

impl TestProverClient {
    fn new(client: ProverGrpcClient<Channel>) -> Self {
        Self {
            client: Mutex::new(client),
        }
    }
}

impl TestStateClient {
    fn new(
        notes: Vec<Note>,
        anchor: BlsScalar,
        opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
    ) -> Self {
        Self {
            notes,
            anchor,
            opening,
        }
    }
}

impl StateClient for TestStateClient {
    type Error = ();

    fn fetch_notes(
        &self,
        _: u64,
        _: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        Ok(self.notes.clone())
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        Ok(self.anchor)
    }

    fn fetch_opening(
        &self,
        _: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {
        Ok(self.opening.clone())
    }

    fn fetch_stake(&self, _pk: &PublicKey) -> Result<(u64, u32), Self::Error> {
        Ok((100, 200))
    }
}

impl ProverClient for TestProverClient {
    type Error = ();

    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<(), Self::Error> {
        let utx = utx.to_bytes().expect("transaction to serialize correctly");
        let request = tonic::Request::new(ExecuteProverRequest { utx });

        let mut prover = self.client.lock().expect("unlock to be successful");
        block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_execute(request).await })
        })
        .expect("successful call");

        Ok(())
    }
    fn request_stct_proof(
        &self,
        _fee: &Fee,
        _crossover: &Crossover,
        _value: u64,
        _blinder: JubJubScalar,
        _address: BlsScalar,
        _signature: Signature,
    ) -> Result<Proof, Self::Error> {
        Ok(Proof::default())
    }

    fn request_wfct_proof(
        &self,
        _commitment: JubJubAffine,
        _value: u64,
        _blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        Ok(Proof::default())
    }
}

#[test_context(TestContext)]
#[tokio::test(flavor = "multi_thread")]
pub async fn prover_walkthrough_uds(ctx: &mut TestContext) {
    let mut rng = rand::thread_rng();
    let client = ProverGrpcClient::new(ctx.channel.clone());

    let wallet = mock_wallet(&mut rng, client, &[5000, 2500, 2500]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    let _ = wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}
