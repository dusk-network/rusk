// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::transaction::TransferPayload;
use crate::Result;

use std::cmp::max;
use std::sync::Arc;

use canonical::{Canon, Sink, Source};
use dusk_abi::ContractState;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_pki::{Ownable, ViewKey};
use dusk_poseidon::tree::PoseidonBranch;
use parking_lot::Mutex;
use phoenix_core::Note;
use rusk_abi::dusk::*;
use rusk_abi::POSEIDON_TREE_DEPTH;
use rusk_recovery_tools::provisioners::DUSK_KEY;
use rusk_vm::{ContractId, GasMeter, NetworkState};
use stake_contract::{Stake, StakeContract};
use transfer_contract::TransferContract;

pub struct RuskState(pub(crate) Arc<Mutex<NetworkState>>);

impl Drop for RuskState {
    fn drop(&mut self) {
        self.0.lock().unstage();
    }
}

impl RuskState {
    pub(crate) fn network(&self) -> Arc<Mutex<NetworkState>> {
        self.0.clone()
    }

    /// Returns the current root of the state tree
    pub fn root(&self) -> [u8; 32] {
        self.0.lock().root()
    }

    /// Accepts the current changes
    pub fn accept(&mut self) {
        self.0.lock().commit()
    }

    /// Finalize the current changes
    pub fn finalize(&mut self) {
        let mut network = self.0.lock();
        network.commit();
        network.push();
    }

    /// Revert to the last finalized state
    pub fn revert(&mut self) {
        self.0.lock().reset()
    }

    /// Fork the underlying network state, returning a new `RuskState`.
    pub fn fork(&self) -> Self {
        let network = self.0.lock().clone();
        Self(Arc::new(Mutex::new(network)))
    }

    /// Executes a transaction on the state via the Transfer Contract
    pub fn execute<R>(
        &mut self,
        block_height: u64,
        transaction: TransferPayload,
        gas_meter: &mut GasMeter,
    ) -> Result<R>
    where
        R: Canon,
    {
        Ok(self.network().lock().transact::<TransferPayload, R>(
            rusk_abi::transfer_contract(),
            block_height,
            transaction,
            gas_meter,
        )?)
    }

    /// Returns a snapshot of a generic contract state. Needs to be casted to
    /// the specific contract type.
    pub fn contract_state<C>(&self, contract_id: &ContractId) -> Result<C>
    where
        C: Canon,
    {
        Ok(self.0.lock().get_contract_cast_state(contract_id)?)
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
        *self.0.lock().get_contract_mut(contract_id)?.state_mut() =
            contract_state;

        Ok(())
    }

    /// Returns a snapshot of the current state of the [`TransferContract`]
    pub fn transfer_contract(&self) -> Result<TransferContract> {
        self.contract_state(&rusk_abi::transfer_contract())
    }

    /// Returns a snapshot of the current state of the [`StakeContract`].
    pub fn stake_contract(&self) -> Result<StakeContract> {
        self.contract_state(&rusk_abi::stake_contract())
    }

    /// Gets the provisioners currently in the stake contract.
    pub fn get_provisioners(&self) -> Result<Vec<(PublicKey, Stake)>> {
        let stake = self.stake_contract()?;
        Ok(stake.stakes()?)
    }

    /// Pushes the coinbase rewards to the state.
    pub fn push_coinbase(
        &mut self,
        block_height: u64,
        dusk_spent: Dusk,
        generator: &PublicKey,
    ) -> Result<()> {
        let (dusk_value, generator_value) =
            coinbase_value(block_height, dusk_spent);

        let generator_key = generator;

        let mut stake = self.stake_contract()?;

        stake.reward(&DUSK_KEY, dusk_value)?;
        stake.reward(generator_key, generator_value)?;

        // SAFETY: this is safe because we know the stake contract exists at the
        // given contract ID
        unsafe {
            self.set_contract_state(&rusk_abi::stake_contract(), &stake)?
        };

        Ok(())
    }

    /// Returns the notes from a given block height and owned by [`ViewKey`],
    /// together with the highest block height found in the tree.
    #[deprecated]
    pub fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<(Vec<Note>, u64)> {
        let mut height = height;

        let notes = self
            .transfer_contract()?
            .leaves_from_height(height)?
            .map(|leaf| *leaf.expect("Failed to fetch leaf from canonical"))
            .map(|leaf| {
                height = max(height, leaf.block_height.into());
                leaf.note
            })
            .filter(|note| vk.owns(note.stealth_address()))
            .collect();

        Ok((notes, height))
    }

    /// Returns the anchor
    pub fn fetch_anchor(&self) -> Result<BlsScalar> {
        Ok(self
            .transfer_contract()?
            .tree()
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
            .tree()
            .opening(*note.pos())
            .map_err(|_| Error::OpeningPositionNotFound(*note.pos()))?
            .ok_or_else(|| Error::OpeningNoteUndefined(*note.pos()))
    }

    /// Returns the stake of a key. If no stake is found, the default is
    /// returned.
    pub fn fetch_stake(&self, pk: &PublicKey) -> Result<Stake> {
        let stake = self
            .stake_contract()?
            .get_stake(pk)?
            .map(|stake| stake.clone())
            .unwrap_or_default();

        Ok(stake)
    }

    /// Returns `true` if any of the nullifier given exists in the current
    /// transfer contract's state.
    pub fn any_nullifier_exists(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<bool, Error> {
        Ok(self.transfer_contract()?.any_nullifier_exists(nullifiers)?)
    }

    /// Takes a slice of nullifiers and returns a vector containing the ones
    /// that already exists in the current transfer contract's state.
    pub fn find_existing_nullifiers(
        &self,
        inputs: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>> {
        Ok(self.transfer_contract()?.find_existing_nullifiers(inputs)?)
    }
}

/// Calculates the value that the coinbase notes should contain.
///
/// 90% of the total value goes to the generator (rounded up).
/// 10% of the total value goes to the Dusk address (rounded down).
const fn coinbase_value(block_height: u64, dusk_spent: u64) -> (Dusk, Dusk) {
    let value = emission_amount(block_height) + dusk_spent;

    let dusk_value = value / 10;
    let generator_value = value - dusk_value;

    (dusk_value, generator_value)
}

/// This implements the emission schedule described in the economic paper.
const fn emission_amount(block_height: u64) -> Dusk {
    match block_height {
        1..=12_500_000 => dusk(16.0),
        12_500_001..=18_750_000 => dusk(12.8),
        18_750_001..=25_000_000 => dusk(9.6),
        25_000_001..=31_250_000 => dusk(8.0),
        31_250_001..=37_500_000 => dusk(6.4),
        37_500_001..=43_750_000 => dusk(4.8),
        43_750_001..=50_000_000 => dusk(3.2),
        50_000_001..=62_500_000 => dusk(1.6),
        _ => dusk(0.0),
    }
}
