// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Mocks of the traits used in the wallet.

use dusk_jubjub::BlsScalar;
use dusk_pki::ViewKey;
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};
use dusk_wallet_core::{NodeClient, Store, UnprovenTransaction, POSEIDON_DEPTH};

#[derive(Debug)]
pub struct TestStore {
    seed: [u8; 64],
}

impl TestStore {
    pub fn new<Rng: RngCore + CryptoRng>(rng: &mut Rng) -> Self {
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
    pub fn new(
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
