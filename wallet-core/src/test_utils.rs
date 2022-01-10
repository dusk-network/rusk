// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Test utilities for use by crates upstream.

use crate::{NodeClient, Store, UnprovenTransaction, Wallet, POSEIDON_DEPTH};

use alloc::vec::Vec;

use dusk_jubjub::{BlsScalar, JubJubScalar};
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Note, NoteType};
use rand_core::{CryptoRng, RngCore};

/// Create a new wallet meant for tests. It includes a client that will always
/// return a random anchor (same every time), and the default opening.
///
/// The number of notes available is determined by `note_values`.
pub fn mock_wallet<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    note_values: &[u64],
) -> Wallet<TestStore, TestNodeClient> {
    let store = TestStore::new(rng);
    let psk = store.retrieve_key(0).unwrap().public_spend_key();

    let notes = new_notes(rng, &psk, note_values);
    let anchor = BlsScalar::random(rng);
    let opening = Default::default();

    let node = TestNodeClient::new(notes, anchor, opening);

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

/// Create a new wallet meant for tests, but wrapping the client in a custom
/// implementation. The given client that will always return a random anchor
/// (same every time), and the default opening.
///
/// The number of notes available is determined by `note_values`.
pub fn mock_wallet_wrap_client<Rng, C, F>(
    rng: &mut Rng,
    note_values: &[u64],
    f: F,
) -> Wallet<TestStore, C>
where
    Rng: RngCore + CryptoRng,
    C: NodeClient,
    F: FnOnce(TestNodeClient) -> C,
{
    let store = TestStore::new(rng);
    let psk = store.retrieve_key(0).unwrap().public_spend_key();

    let notes = new_notes(rng, &psk, note_values);
    let anchor = BlsScalar::random(rng);
    let opening = Default::default();

    let node = f(TestNodeClient::new(notes, anchor, opening));
    Wallet::new(store, node)
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

/// A node client that always returns the same notes, anchor, and opening.
#[derive(Debug, Clone)]
pub struct TestNodeClient {
    notes: Vec<Note>,
    anchor: BlsScalar,
    opening: PoseidonBranch<POSEIDON_DEPTH>,
}

impl TestNodeClient {
    /// Create a new node given the notes, anchor, and opening we will return.
    fn new(
        notes: Vec<Note>,
        anchor: BlsScalar,
        opening: PoseidonBranch<POSEIDON_DEPTH>,
    ) -> Self {
        Self {
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
    ) -> Result<PoseidonBranch<POSEIDON_DEPTH>, Self::Error> {
        Ok(self.opening.clone())
    }

    fn request_proof(
        &self,
        _: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error> {
        Ok(Proof::default())
    }
}
