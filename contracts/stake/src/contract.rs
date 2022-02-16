// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical_derive::Canon;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use microkelvin::First;
use phoenix_core::Note;

use alloc::vec::Vec;

#[cfg(feature = "transaction")]
mod transaction;

#[derive(Debug, Default, Clone, Canon)]
pub struct StakeContract {
    staked: Map<PublicKey, Stake>,
    last_created: Map<PublicKey, BlockHeight>,
}

impl StakeContract {
    pub fn get_stake(&self, key: &PublicKey) -> Result<Stake, Error> {
        self.staked
            .get(key)?
            .map(|s| *s)
            .ok_or(Error::StakeNotFound)
    }

    pub fn push_stake(
        &mut self,
        pk: PublicKey,
        stake: Stake,
        block_height: BlockHeight,
    ) -> Result<(), Error> {
        let exists = self.staked.get(&pk)?.is_some();
        if exists {
            return Err(Error::StakeAlreadyExists);
        }

        // `created_at` must never be larger than the block height.
        if stake.created_at() > block_height {
            return Err(Error::InvalidCreatedAt);
        }

        // A last_created entry is left when the stake is removed to be able to
        // make this check. We try to remove it, and if it exists it must not be
        // larger or equal to the given `created_at`.
        if let Some(created_at) = self.last_created.get(&pk)? {
            if stake.created_at() <= *created_at {
                return Err(Error::InvalidCreatedAt);
            }
        }
        self.last_created.remove(&pk)?;

        self.last_created.insert(pk, stake.created_at())?;
        self.staked.insert(pk, stake)?;

        Ok(())
    }

    pub fn remove_stake(&mut self, pk: &PublicKey) -> Result<(), Error> {
        let removed = self.staked.remove(pk)?.is_some();
        if !removed {
            return Err(Error::StakeNotFound);
        }

        Ok(())
    }

    pub fn is_staked(
        &self,
        block_height: u64,
        key: &PublicKey,
    ) -> Result<bool, Error> {
        let is_staked = self
            .staked
            .get(key)?
            .filter(|s| s.is_valid(block_height))
            .is_some();

        Ok(is_staked)
    }

    /// Gets a vector of all public keys and stakes.
    pub fn stakes(&self) -> Result<Vec<(PublicKey, Stake)>, Error> {
        let mut stakes = Vec::new();

        if let Some(branch) = self.staked.first()? {
            for leaf in branch {
                let leaf = leaf?;
                stakes.push((leaf.key, leaf.val));
            }
        }

        Ok(stakes)
    }

    pub fn stake_sign_message(value: u64, created_at: BlockHeight) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);

        bytes.extend(value.to_bytes());
        bytes.extend(created_at.to_le_bytes());

        bytes
    }

    pub fn withdraw_sign_message(stake: &Stake, note: &Note) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24 + Note::SIZE);

        bytes.extend(stake.value().to_le_bytes());
        bytes.extend(stake.eligibility().to_le_bytes());
        bytes.extend(stake.created_at().to_le_bytes());
        bytes.extend(note.to_bytes());

        bytes
    }
}
