// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_wallet_core::{ProverClient, StateClient, UnprovenTransaction, POSEIDON_TREE_DEPTH};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::{ViewKey, PublicKey};
use dusk_schnorr::Signature;
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Note, Crossover, Fee};

#[derive(Debug)]
pub(crate) enum ProverError {

}

pub(crate) struct Prover {

}

impl ProverClient for Prover {

    /// Error returned by the prover client.
    type Error = ProverError;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(&self, utx: &UnprovenTransaction) -> Result<(), Self::Error> {
        unimplemented!();
    }

    /// Requests an STCT proof.
    fn request_stct_proof(&self, fee: &Fee, crossover: &Crossover, value: u64, blinder: JubJubScalar, 
                          address: BlsScalar, signature: Signature) -> Result<Proof, Self::Error> {
        unimplemented!();

    }

    /// Request a WFCT proof.
    fn request_wfct_proof( &self, commitment: JubJubAffine, value: u64, blinder: JubJubScalar) -> Result<Proof, Self::Error> {
        unimplemented!();
    }

}

#[derive(Debug)]
pub(crate) enum StateError {

}

pub(crate) struct State {

}

/// Types that are clients of the state API.
impl StateClient for State {

    /// Error returned by the node client.
    type Error = StateError;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(&self, height: u64, vk: &ViewKey) -> Result<Vec<Note>, Self::Error> {
        unimplemented!()
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        unimplemented!()
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(&self, note: &Note) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {
        unimplemented!()
    }

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<(u64, u32), Self::Error> {
        unimplemented!()
    }
}
