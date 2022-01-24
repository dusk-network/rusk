// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::Result;
use dusk_bls12_381::BlsScalar;
use dusk_pki::{Ownable, ViewKey};
use dusk_poseidon::tree::PoseidonBranch;
use microkelvin::{Backend, BackendCtor};
use phoenix_core::Note;
use rusk_abi::{self, POSEIDON_TREE_DEPTH};
use rusk_vm::{NetworkState, NetworkStateId};
use transfer_contract::TransferContract;

pub struct RuskState(pub(crate) NetworkState);

impl RuskState {
    /// Returns a reference to the underlying [`NetworkState`]
    #[inline(always)]
    pub fn inner(&self) -> &NetworkState {
        &self.0
    }

    /// Returns a mutable reference to the underlying [`NetworkState`]
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut NetworkState {
        &mut self.0
    }

    /// Returns the current root of the state tree
    pub fn root(&self) -> [u8; 32] {
        self.0.root()
    }

    pub fn persist<B>(
        &mut self,
        ctor: &BackendCtor<B>,
    ) -> Result<NetworkStateId>
    where
        B: 'static + Backend,
    {
        Ok(self.0.persist(ctor)?)
    }

    pub fn commit(&mut self) {
        self.0.commit()
    }

    /// Returns the current state of the [`TransferContract`]
    pub fn transfer_contract(&self) -> Result<TransferContract> {
        Ok(self
            .0
            .get_contract_cast_state(&rusk_abi::transfer_contract())?)
    }

    /// Returns all the notes from a given block height
    fn notes(&self, height: u64) -> Result<Vec<Note>> {
        Ok(self
            .transfer_contract()?
            .notes_from_height(height)?
            .map(|note| *note.expect("Failed to fetch note from canonical"))
            .collect())
    }

    /// Returns the note at a given block height and [`ViewKey`]
    pub fn fetch_notes(&self, height: u64, vk: &ViewKey) -> Result<Vec<Note>> {
        Ok(self
            .notes(height)?
            .iter()
            .filter(|n| vk.owns(n.stealth_address()))
            .copied()
            .collect())
    }

    /// Returns the anchor
    pub fn fetch_anchor(&self) -> Result<BlsScalar> {
        Ok(self
            .transfer_contract()?
            .notes()
            .inner()
            .root()
            .unwrap_or_default())
    }

    /// Returns the opening
    pub fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>> {
        self.transfer_contract()?
            .notes()
            .opening(*note.pos())
            .map_err(|_| Error::OpeningPositionNotFound(*note.pos()))?
            .ok_or_else(|| Error::OpeningNoteUndefined(*note.pos()))
    }
}
