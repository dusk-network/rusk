// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::Result;

use std::ops::Deref;

use canonical::{Canon, Sink, Source};
use dusk_abi::ContractState;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, PublicKey, PublicSpendKey, ViewKey};
use dusk_poseidon::tree::PoseidonBranch;
use microkelvin::{Backend, BackendCtor};
use phoenix_core::Note;
use rusk_abi::{self, POSEIDON_TREE_DEPTH};
use rusk_vm::{ContractId, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract};
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

    pub(crate) fn persist<B>(
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

    /// Returns a generic contract state. Needs to be casted to the specific
    /// contract type.
    pub fn contract_state(
        &self,
        contract_id: &ContractId,
    ) -> Result<ContractState> {
        Ok(self.0.get_contract_cast_state(contract_id)?)
    }

    /// Set the contract state for the given Contract Id.
    ///
    /// # Safety
    ///
    /// This function will corrupt the state if the contract state given is
    /// not the same type as the one stored in the state at the address
    /// provided; and the subsequent contract's call will fail.
    pub unsafe fn set_contract_state<C>(
        &mut self,
        contract_id: &ContractId,
        state: &C,
    ) -> Result<()>
    where
        C: Canon,
    {
        const PAGE_SIZE: usize = 1024 * 64;
        let mut bytes = [0u8; PAGE_SIZE];
        let mut sink = Sink::new(&mut bytes[..]);
        ContractState::from_canon(state).encode(&mut sink);
        let mut source = Source::new(&bytes[..]);
        let contract_state = ContractState::decode(&mut source)?;
        *self.0.get_contract_mut(contract_id)?.state_mut() = contract_state;

        Ok(())
    }

    /// Returns the current state of the [`TransferContract`]
    pub fn transfer_contract(&self) -> Result<TransferContract> {
        Ok(self
            .0
            .get_contract_cast_state(&rusk_abi::transfer_contract())?)
    }

    /// Returns the current state of the [`StakeContract`].
    pub fn stake_contract(&self) -> Result<StakeContract> {
        Ok(self
            .0
            .get_contract_cast_state(&rusk_abi::stake_contract())?)
    }

    /// Mints two notes into the transfer contract state, to pay gas fees.
    pub fn mint(
        &mut self,
        block_height: u64,
        gas_spent: u64,
        generator: Option<&PublicSpendKey>,
    ) -> Result<(Note, Note)> {
        let (dusk_value, generator_value) =
            coinbase_value(block_height, gas_spent);

        let mut transfer = self.transfer_contract()?;
        let notes = transfer.mint(
            block_height,
            dusk_value,
            generator_value,
            generator,
        )?;

        // SAFETY: this is safe because we know the transfer contract exists at
        // the given contract ID.
        unsafe {
            self.set_contract_state(&rusk_abi::transfer_contract(), &transfer)?
        };

        Ok(notes)
    }

    /// Pushes two notes onto the state, checking their amounts to be correct
    /// and updates it.
    pub fn push_coinbase(
        &mut self,
        block_height: u64,
        gas_spent: u64,
        coinbase: (Note, Note),
    ) -> Result<()> {
        let mut transfer = self.transfer_contract()?;

        let dusk_value = coinbase.0.value(None)?;
        let generator_value = coinbase.1.value(None)?;

        let (expected_dusk, expected_generator) =
            coinbase_value(block_height, gas_spent);

        if dusk_value != expected_dusk {
            return Err(Error::CoinbaseValue(dusk_value, expected_dusk));
        }
        if generator_value != expected_generator {
            return Err(Error::CoinbaseValue(
                generator_value,
                expected_generator,
            ));
        }

        transfer.push_note(block_height, coinbase.0)?;
        transfer.push_note(block_height, coinbase.1)?;

        transfer.update_root()?;

        // SAFETY: this is safe because we know the transfer contract exists at
        // the given contract ID.
        unsafe {
            self.set_contract_state(&rusk_abi::transfer_contract(), &transfer)?
        };

        Ok(())
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

    /// Returns the stake of a key.
    pub fn fetch_stake(&self, pk: &PublicKey) -> Result<Stake> {
        self.stake_contract()?
            .staked
            .get(&pk.to_bytes())?
            .map(|s| *s.deref())
            .ok_or_else(|| Error::StakeNotFound(*pk))
    }
}

/// Calculates the value that the coinbase notes should contain.
///
/// 90% of the total value goes to the generator (rounded up).
/// 10% of the total value goes to the Dusk address (rounded down).
fn coinbase_value(block_height: u64, gas_spent: u64) -> (u64, u64) {
    let value = emission_amount(block_height) + gas_spent;

    let dusk_value = value / 10;
    let generator_value = value - dusk_value;

    (dusk_value, generator_value)
}

/// This implements the emission schedule described in the economic paper.
fn emission_amount(block_height: u64) -> u64 {
    match block_height {
        1..=12_500_000 => 16_000_000,
        12_500_001..=18_750_000 => 12_800_000,
        18_750_001..=25_000_000 => 9_600_000,
        25_000_001..=31_250_000 => 8_000_000,
        31_250_001..=37_500_000 => 6_400_000,
        37_500_001..=43_750_000 => 4_800_000,
        43_750_001..=50_000_000 => 3_200_000,
        50_000_001..=62_500_000 => 1_600_000,
        _ => 0,
    }
}
