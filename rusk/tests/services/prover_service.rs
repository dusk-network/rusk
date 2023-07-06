// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::*;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::DeserializableSlice;
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    ProverClient as WalletProverClient, StakeInfo, StateClient, Store,
    Transaction, UnprovenTransaction, Wallet, POSEIDON_TREE_DEPTH,
};
use parking_lot::Mutex;
use phoenix_core::{Crossover, Fee};
use phoenix_core::{Note, NoteType};
use poseidon_merkle::{Item, Opening as PoseidonOpening, Tree};
use rand::{CryptoRng, RngCore};
use rusk::services::prover::{ProverServer, RuskProver};
use rusk_schema::prover_client::ProverClient;
use rusk_schema::ExecuteProverRequest;
use tonic::transport::Channel;
use tonic::transport::Server;

/// Create a new wallet meant for tests. It includes a client that will always
/// return a random anchor (same every time), and the default opening.
///
/// The number of notes available is determined by `note_values`.
fn mock_wallet<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    client: ProverClient<Channel>,
    note_values: &[u64],
) -> Wallet<TestStore, TestStateClient, TestWalletProverClient> {
    let store = TestStore::new(rng);
    let psk = store.retrieve_ssk(0).unwrap().public_spend_key();

    let notes = new_notes(rng, &psk, note_values);
    let anchor = BlsScalar::random(rng);

    const POS: u64 = 42;
    let mut tree = Tree::new();
    tree.insert(
        POS,
        Item {
            hash: BlsScalar::zero(),
            data: (),
        },
    );
    let opening = tree.opening(POS).unwrap();

    let node = TestWalletProverClient::new(client);
    let state = TestStateClient::new(notes, anchor, opening, vec![]);

    Wallet::new(store, state, node)
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
struct TestWalletProverClient {
    client: Mutex<ProverClient<Channel>>,
}

impl TestWalletProverClient {
    fn new(client: ProverClient<Channel>) -> Self {
        Self {
            client: Mutex::new(client),
        }
    }
}

impl WalletProverClient for TestWalletProverClient {
    type Error = ();

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error> {
        let utx_bytes = utx.to_var_bytes();
        let request =
            tonic::Request::new(ExecuteProverRequest { utx: utx_bytes });

        let mut prover = self.client.lock();
        let response = prover
            .prove_execute(request)
            .wait()
            .expect("successful call")
            .into_inner();

        let proof = Proof::from_slice(&response.proof).expect("valid proof");
        let tx = utx.clone().prove(proof);

        Ok(tx)
    }

    /// Requests an STCT proof.
    fn request_stct_proof(
        &self,
        _fee: &Fee,
        _crossover: &Crossover,
        _value: u64,
        _blinder: JubJubScalar,
        _address: BlsScalar,
        _signature: Signature,
    ) -> Result<Proof, Self::Error> {
        unimplemented!();
    }

    /// Request a WFCT proof.
    fn request_wfct_proof(
        &self,
        _commitment: JubJubAffine,
        _value: u64,
        _blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        unimplemented!();
    }
}

/// An in-memory seed store.
#[derive(Debug)]
pub struct TestStateClient {
    notes: Vec<Note>,
    anchor: BlsScalar,
    opening: PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>,
    nullifiers: Vec<BlsScalar>,
}

impl TestStateClient {
    fn new(
        notes: Vec<Note>,
        anchor: BlsScalar,
        opening: PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>,
        nullifiers: Vec<BlsScalar>,
    ) -> Self {
        Self {
            notes,
            anchor,
            opening,
            nullifiers,
        }
    }
}

impl StateClient for TestStateClient {
    type Error = ();

    fn fetch_notes(
        &self,
        _: &ViewKey,
    ) -> Result<Vec<(Note, u64)>, Self::Error> {
        Ok(self.notes.iter().map(|note| (note.clone(), 0)).collect())
    }

    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        Ok(self.anchor)
    }

    fn fetch_existing_nullifiers(
        &self,
        _nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        Ok(self.nullifiers.clone())
    }

    fn fetch_opening(
        &self,
        _: &Note,
    ) -> Result<PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>, Self::Error> {
        Ok(self.opening.clone())
    }

    fn fetch_stake(&self, _pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        unimplemented!();
    }
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
pub async fn prover_walkthrough_uds() {
    let (channel, incoming) = setup().await;

    tokio::spawn(async move {
        Server::builder()
            .add_service(ProverServer::new(RuskProver::default()))
            .serve_with_incoming(incoming)
            .await
    });

    let mut rng = rand::thread_rng();
    let client = ProverClient::new(channel);

    let wallet = mock_wallet(&mut rng, client, &[5000, 2500, 2500]);

    let send_psk = wallet.public_spend_key(0).unwrap();
    let recv_psk = wallet.public_spend_key(1).unwrap();

    let ref_id = BlsScalar::random(&mut rng);
    let _ = wallet
        .transfer(&mut rng, 0, &send_psk, &recv_psk, 100, 100, 1, ref_id)
        .expect("Transaction creation to be successful");
}
