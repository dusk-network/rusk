// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, RwLock};

use crate::common::block::Block as BlockAwait;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::ViewKey;
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    self as wallet, StakeInfo, Store, Transaction as PhoenixTransaction,
    UnprovenTransaction,
};
use futures::StreamExt;
use phoenix_core::transaction::TRANSFER_TREE_DEPTH;
use phoenix_core::{Crossover, Fee, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk::{Error, Result, Rusk};
use rusk_prover::prover::{A, STCT_INPUT_LEN, WFCT_INPUT_LEN};
use rusk_prover::{LocalProver, Prover};
use tracing::info;

#[derive(Debug, Clone)]
pub struct TestStore;

impl Store for TestStore {
    type Error = ();

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok([0; 64])
    }
}

#[derive(Clone)]
pub struct TestStateClient {
    pub rusk: Rusk,
    pub cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
}

impl std::fmt::Debug for TestStateClient {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl wallet::StateClient for TestStateClient {
    type Error = Error;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        vk: &ViewKey,
    ) -> Result<Vec<(Note, u64)>, Self::Error> {
        let cache_read = self.cache.read().unwrap();
        let mut vk_cache = if cache_read.contains_key(&vk.to_bytes().to_vec()) {
            cache_read.get(&vk.to_bytes().to_vec()).unwrap().clone()
        } else {
            DummyCacheItem::default()
        };

        info!("Requesting notes from height {}", vk_cache.last_height);
        let vk_bytes = vk.to_bytes();

        let stream = self
            .rusk
            .get_notes(vk_bytes.as_ref(), vk_cache.last_height)
            .wait()?;

        let response_notes = stream.collect::<Vec<(Note, u64)>>().wait();

        for (note, block_height) in response_notes {
            // Filter out duplicated notes and update the last
            vk_cache.add(note, block_height)
        }
        drop(cache_read);
        self.cache
            .write()
            .unwrap()
            .insert(vk.to_bytes().to_vec(), vk_cache.clone());

        Ok(vk_cache.notes)
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        self.rusk.tree_root()
    }

    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        self.rusk.existing_nullifiers(&nullifiers.to_vec())
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>, Self::Error> {
        self.rusk
            .tree_opening(*note.pos())?
            .ok_or(Error::OpeningPositionNotFound(*note.pos()))
    }

    fn fetch_stake(&self, _pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let stake = self
            .rusk
            .provisioners(None)?
            .find(|(pk, _)| pk == _pk)
            .map(|(_, stake)| StakeInfo {
                amount: stake.amount,
                counter: stake.counter,
                reward: stake.reward,
            })
            .unwrap_or_default();
        Ok(stake)
    }
}

#[derive(Default)]
pub struct TestProverClient {
    pub prover: LocalProver,
}

impl Debug for TestProverClient {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl wallet::ProverClient for TestProverClient {
    type Error = Error;
    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<PhoenixTransaction, Self::Error> {
        let utx_bytes = &utx.to_var_bytes()[..];
        let proof = self.prover.prove_execute(utx_bytes)?;
        info!("UTX: {}", hex::encode(utx_bytes));
        let proof = Proof::from_slice(&proof).map_err(Error::Serialization)?;
        let tx = utx.clone().prove(proof);

        //Propagate is not required yet

        Ok(tx)
    }
    /// Requests an STCT proof.
    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0u8; STCT_INPUT_LEN];
        let mut writer = &mut buf[..];

        writer.write_all(&fee.to_bytes())?;
        writer.write_all(&crossover.to_bytes())?;
        writer.write_all(&value.to_bytes())?;
        writer.write_all(&blinder.to_bytes())?;
        writer.write_all(&address.to_bytes())?;
        writer.write_all(&signature.to_bytes())?;

        let proof = self.prover.prove_stct(&buf)?;
        Proof::from_slice(&proof[..]).map_err(Error::Serialization)
    }

    /// Request a WFCT proof.
    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0u8; WFCT_INPUT_LEN];
        let mut writer = &mut buf[..];

        writer.write_all(&commitment.to_bytes())?;
        writer.write_all(&value.to_bytes())?;
        writer.write_all(&blinder.to_bytes())?;

        let proof = self.prover.prove_wfct(&buf)?;
        Proof::from_slice(&proof[..]).map_err(Error::Serialization)
    }
}

#[derive(Default, Debug, Clone)]
pub struct DummyCacheItem {
    notes: Vec<(Note, u64)>,
    last_height: u64,
}

impl DummyCacheItem {
    fn add(&mut self, note: Note, block_height: u64) {
        if !self.notes.contains(&(note, block_height)) {
            self.notes.push((note, block_height));
            self.last_height = block_height;
        }
    }
}
