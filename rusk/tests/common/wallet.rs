// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod test_wallet;

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use crate::common::block::Block as BlockAwait;

use dusk_bytes::Serializable;
use dusk_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    stake::StakeData,
    transfer::{
        moonlight::AccountData,
        phoenix::{Note, NoteOpening, ViewKey},
    },
    BlsScalar,
};
use futures::StreamExt;
use rusk::{Error, Result, Rusk};
use test_wallet::{self as wallet, Store};
use tracing::info;

#[allow(unused_imports)]
pub use test_wallet::Wallet;

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

    /// Fetch the current root of the state.
    fn fetch_root(&self) -> Result<BlsScalar, Self::Error> {
        self.rusk.tree_root()
    }

    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        self.rusk.existing_nullifiers(&nullifiers.to_vec())
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(&self, note: &Note) -> Result<NoteOpening, Self::Error> {
        self.rusk
            .tree_opening(*note.pos())?
            .ok_or(Error::OpeningPositionNotFound(*note.pos()))
    }

    fn fetch_stake(&self, pk: &BlsPublicKey) -> Result<StakeData, Self::Error> {
        let stake = self.rusk.provisioner(pk)?.unwrap_or(StakeData::EMPTY);
        Ok(stake)
    }

    fn fetch_account(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, Self::Error> {
        let account = self.rusk.account(pk)?;
        Ok(account)
    }

    fn fetch_chain_id(&self) -> Result<u8, Self::Error> {
        let chain_id = self.rusk.chain_id()?;
        Ok(chain_id)
    }
}

#[derive(Default, Debug, Clone)]
pub struct DummyCacheItem {
    notes: Vec<(Note, u64)>,
    last_height: u64,
}

impl DummyCacheItem {
    fn add(&mut self, note: Note, block_height: u64) {
        if !self.notes.contains(&(note.clone(), block_height)) {
            self.notes.push((note.clone(), block_height));
            self.last_height = block_height;
        }
    }
}
