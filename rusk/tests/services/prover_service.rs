// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TestContext;

use std::sync::Mutex;

use dusk_bytes::DeserializableSlice;
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_wallet_core::{
    NodeClient, Store, UnprovenTransaction, Wallet, POSEIDON_TREE_DEPTH,
};
use phoenix_core::{Note, NoteType};
use rand::{CryptoRng, RngCore};
use rusk::services::rusk_proto::prover_client::ProverClient;
use rusk::services::rusk_proto::ProverRequest;
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
    client: ProverClient<Channel>,
    note_values: &[u64],
) -> Wallet<TestStore, TestNodeClient> {
    let store = TestStore::new(rng);
    let psk = store.retrieve_key(0).unwrap().public_spend_key();

    let notes = new_notes(rng, &psk, note_values);
    let anchor = BlsScalar::random(rng);
    let opening = Default::default();

    let node = TestNodeClient::new(client, notes, anchor, opening);

    Wallet::new(store, node)
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
struct TestNodeClient {
    client: Mutex<ProverClient<Channel>>,

    notes: Vec<Note>,
    anchor: BlsScalar,
    opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
}

impl TestNodeClient {
    fn new(
        client: ProverClient<Channel>,
        notes: Vec<Note>,
        anchor: BlsScalar,
        opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
    ) -> Self {
        Self {
            client: Mutex::new(client),
            notes,
            anchor,
            opening,
        }
    }
}

impl NodeClient for TestNodeClient {
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

    fn request_proof(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error> {
        let utx = utx.to_bytes().expect("transaction to serialize correctly");
        let request = tonic::Request::new(ProverRequest { utx });

        let mut prover = self.client.lock().expect("unlock to be successful");
        let proof = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove(request).await })
        })
        .expect("successful call")
        .into_inner()
        .proof;

        let proof = Proof::from_slice(&proof).expect("valid proof");
        Ok(proof)
    }
}

#[test_context(TestContext)]
#[tokio::test(flavor = "multi_thread")]
pub async fn prover_walkthrough_uds(ctx: &mut TestContext) {
    let mut rng = rand::thread_rng();
    let client = ProverClient::new(ctx.channel.clone());

    let wallet = mock_wallet(&mut rng, client, &[5000, 2500, 2500]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    let _ = wallet
        .create_transfer_tx(
            &mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id,
        )
        .expect("Transaction creation to be successful");
}
