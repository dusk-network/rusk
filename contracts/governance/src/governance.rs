// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::iter;

use alloc::vec;
use alloc::vec::Vec;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey as BlsPublicKey, Signature};
use dusk_bytes::Serializable;

use crate::collection::Collection;
use crate::*;

#[derive(Debug, Default, Clone, Canon)]
pub struct GovernanceContract {
    pub(crate) seeds: Collection<BlsScalar, ()>,
    pub(crate) balances: Collection<PublicKey, u64>,
    pub(crate) whitelist: Collection<PublicKey, ()>,
    pub(crate) paused: bool,
    pub(crate) total_supply: u64,
}

impl GovernanceContract {
    /// Authority of the contract
    ///
    /// Will have to be defined in the constant space so the bytecode of the
    /// contract will be changed as the authority does
    pub const AUTHORITY: &[u8; 96] = include_bytes!(concat!(
        env!("RUSK_PROFILE_PATH"),
        "/governance_authority.cpk"
    ));

    fn validate_seed(
        &mut self,
        arguments: Vec<BlsScalar>,
        signature: Signature,
    ) -> Result<(), Error> {
        // Cannot construct BlsPublicKey in a const context that's why we
        // construct it in the function body.
        let authority = BlsPublicKey::from_bytes(Self::AUTHORITY)
            .map_err(|_| Error::InvalidPublicKey)?;

        let seed = arguments[0];

        if self.seeds.get(&seed)?.is_some() {
            return Err(Error::SeedAlreadyUsed);
        }

        #[cfg(target_arch = "wasm32")]
        if !rusk_abi::verify_bls_sign(
            signature,
            Self::AUTHORITY,
            authority,
            rusk_abi::poseidon_hash(arguments).to_bytes().to_vec(),
        ) {
            return Err(Error::InvalidSignature);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if authority
            .verify(
                &signature,
                &dusk_poseidon::sponge::hash(&arguments).to_bytes(),
            )
            .is_ok()
        {
            return Err(Error::InvalidSignature);
        }

        self.seeds.insert(seed, ())?;

        Ok(())
    }

    fn check_pause(&self) -> Result<(), Error> {
        (!self.paused).then_some(()).ok_or(Error::ContractIsPaused)
    }

    // to keep code consistent with other collections, we supress deref warnings
    // as its not implemented for other when we switch features.
    fn is_allowed(&self, address: &PublicKey) -> Result<(), Error> {
        self.whitelist
            .get(address)?
            .copied()
            .ok_or(Error::AddressIsNotWhitelisted)
    }

    pub fn pause(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
    ) -> Result<(), Error> {
        self.validate_seed(
            vec![seed, BlsScalar::from(TX_PAUSE as u64)],
            signature,
        )?;

        self.paused = true;

        Ok(())
    }

    pub fn unpause(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
    ) -> Result<(), Error> {
        self.validate_seed(
            vec![seed, BlsScalar::from(TX_UNPAUSE as u64)],
            signature,
        )?;
        self.paused = false;

        Ok(())
    }

    pub fn allow(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
    ) -> Result<(), Error> {
        self.validate_seed(
            iter::once([seed, BlsScalar::from(TX_ALLOW as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .collect(),
            signature,
        )?;
        self.whitelist.insert(address, ())?;

        Ok(())
    }

    pub fn block(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
    ) -> Result<(), Error> {
        self.validate_seed(
            iter::once([seed, BlsScalar::from(TX_BLOCK as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .collect(),
            signature,
        )?;
        self.whitelist.remove(&address)?;

        Ok(())
    }

    pub fn mint(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.validate_seed(
            iter::once([seed, BlsScalar::from(TX_MINT as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .chain(iter::once(BlsScalar::from(value)))
                .collect(),
            signature,
        )?;
        self.check_pause()?;
        self.is_allowed(&address)?;

        self.total_supply = self
            .total_supply
            .checked_add(value)
            .ok_or(Error::BalanceOverflow)?;

        let value = self
            .balances
            .get(&address)?
            .copied()
            .unwrap_or(0)
            .checked_add(value)
            .ok_or(Error::BalanceOverflow)?;

        self.balances.insert(address, value)?;

        Ok(())
    }

    pub fn burn(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.validate_seed(
            iter::once([seed, BlsScalar::from(TX_BURN as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .chain(iter::once(BlsScalar::from(value)))
                .collect(),
            signature,
        )?;
        self.check_pause()?;

        self.total_supply = self.total_supply.saturating_sub(value);

        let value = self
            .balances
            .get(&address)?
            .copied()
            .unwrap_or(0)
            .checked_sub(value)
            .ok_or(Error::InsufficientBalance)?;

        if value == 0 {
            self.balances.remove(&address)?;
        } else {
            self.balances.insert(address, value)?;
        }

        Ok(())
    }

    pub fn transfer(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        batch: Vec<Transfer>,
    ) -> Result<(), Error> {
        self.validate_seed(
            iter::once([seed, BlsScalar::from(TX_TRANSFER as u64)])
                .flatten()
                .chain(batch.iter().flat_map(Transfer::as_scalars))
                .collect(),
            signature,
        )?;
        self.check_pause()?;

        for Transfer {
            from, to, amount, ..
        } in batch
        {
            self.is_allowed(&from)?;

            let mut base = self.balances.get(&from)?.copied().unwrap_or(0);

            if base < amount {
                let remaining = amount - base;

                self.mint(seed, signature, from, remaining)?;

                base = 0;
            } else {
                base -= amount;
            }

            let target = self
                .balances
                .get(&to)?
                .copied()
                .unwrap_or(0)
                .checked_add(amount)
                .ok_or(Error::BalanceOverflow)?;

            if base == 0 {
                self.balances.remove(&from)?;
            } else {
                self.balances.insert(from, base)?;
            }

            if target == 0 {
                self.balances.remove(&to)?;
            } else {
                self.balances.insert(to, target)?;
            }
        }

        Ok(())
    }
}
