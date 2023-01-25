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

#[derive(Debug, Clone, Default, Canon)]
pub struct GovernanceContract {
    pub(crate) seeds: Collection<BlsScalar, ()>,
    pub(crate) balances: Collection<PublicKey, u64>,
    pub(crate) whitelist: Collection<PublicKey, ()>,
    pub(crate) paused: bool,
    pub(crate) total_supply: u64,
    // we use BlsPublicKey or dusk_bls12_381_sign::PublicKey and not a
    // dusk_pki::PublicKey because of our verification method
    pub broker: BlsPublicKey,
    pub authority: BlsPublicKey,
}

/// Use `GovernanceContract::default()` for instance.
impl GovernanceContract {
    // convert a buffer to a BlsPublicKey
    fn bls_public_key(key: &[u8; 96]) -> Result<BlsPublicKey, Error> {
        BlsPublicKey::from_bytes(key).map_err(|_| Error::InvalidPublicKey)
    }

    fn validate_seed(
        &mut self,
        arguments: Vec<BlsScalar>,
        signature: Signature,
    ) -> Result<(), Error> {
        let seed = arguments[0];

        if self.seeds.get(&seed)?.is_some() {
            return Err(Error::SeedAlreadyUsed);
        }

        #[cfg(target_arch = "wasm32")]
        if !rusk_abi::verify_bls_sign(
            signature,
            self.authority,
            rusk_abi::poseidon_hash(arguments).to_bytes().to_vec(),
        ) {
            return Err(Error::InvalidSignature);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self
            .authority
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
    /// Update the authority public address of the Governance Contract
    pub fn update_authority(&mut self, key: &[u8; 96]) -> Result<(), Error> {
        self.authority = Self::bls_public_key(key)?;

        Ok(())
    }
    /// Update the broker public address of the Governance Contract
    pub fn update_broker(&mut self, key: &[u8; 96]) -> Result<(), Error> {
        self.broker = Self::bls_public_key(key)?;

        Ok(())
    }
}

// write unit test for this module
#[cfg(test)]
mod tests {
    use dusk_bls12_381::G2Affine;

    use super::*;

    #[test]
    fn check_update_authority() {
        let mut contract = GovernanceContract::default();

        assert_eq!(contract.authority, BlsPublicKey::default());
        let key = G2Affine::generator().to_bytes();

        contract.update_authority(&key).unwrap();

        assert_eq!(
            contract.authority,
            GovernanceContract::bls_public_key(&key).unwrap()
        );
    }
    #[test]
    fn check_update_broker() {
        let mut contract = GovernanceContract::default();

        assert_eq!(contract.broker, BlsPublicKey::default());
        let key = G2Affine::generator().to_bytes();

        contract.update_broker(&key).unwrap();

        assert_eq!(
            contract.broker,
            GovernanceContract::bls_public_key(&key).unwrap()
        );
    }
}
