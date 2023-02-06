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

use crate::collection::{Map, Set};
use crate::*;

#[derive(Debug, Clone, Default, Canon)]
pub struct GovernanceContract {
    pub(crate) seeds: Set<BlsScalar>,
    pub(crate) balances: Map<PublicKey, u64>,
    pub(crate) whitelist: Set<PublicKey>,
    pub(crate) paused: bool,
    pub(crate) total_supply: u64,
    // we use BlsPublicKey or dusk_bls12_381_sign::PublicKey and not a
    // dusk_pki::PublicKey because of our verification method
    //
    // They need to be public so they can be accessed from the host environment
    // (rusk). Once contract deployment stragery will be implemented, this will
    // change.
    pub broker: BlsPublicKey,
    pub authority: BlsPublicKey,
}

/// Use `GovernanceContract::default()` for instance.
impl GovernanceContract {
    fn assert_seed(
        &mut self,
        arguments: Vec<BlsScalar>,
        signature: Signature,
    ) -> Result<(), Error> {
        let seed = arguments[0];

        if self.seeds.contains(&seed) {
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

        self.seeds.insert(seed);

        Ok(())
    }

    fn assert_pause(&self) -> Result<(), Error> {
        if self.paused {
            Err(Error::ContractIsPaused)
        } else {
            Ok(())
        }
    }

    fn assert_address(&self, address: &PublicKey) -> Result<(), Error> {
        self.whitelist
            .get(address)
            .ok_or(Error::AddressIsNotWhitelisted)?;

        Ok(())
    }

    fn balance_add(
        &mut self,
        address: &PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        if let Some(balance) = self.balances.get_mut(address) {
            *balance =
                balance.checked_add(value).ok_or(Error::BalanceOverflow)?;
        } else {
            self.balances.insert(*address, value);
        }

        Ok(())
    }

    fn balance_sub(&mut self, address: &PublicKey, value: u64) -> u64 {
        match self.balances.get_mut(address) {
            Some(balance) if *balance < value => {
                let remaining = value - *balance;
                *balance = 0;

                remaining
            }
            Some(balance) => {
                *balance -= value;
                0
            }
            None => 0,
        }
    }

    pub fn pause(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
    ) -> Result<(), Error> {
        self.assert_seed(
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
        self.assert_seed(
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
        self.assert_seed(
            iter::once([seed, BlsScalar::from(TX_ALLOW as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .collect(),
            signature,
        )?;

        self.whitelist.insert(address);

        Ok(())
    }

    pub fn block(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
    ) -> Result<(), Error> {
        self.assert_seed(
            iter::once([seed, BlsScalar::from(TX_BLOCK as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .collect(),
            signature,
        )?;

        self.whitelist.remove(&address);

        Ok(())
    }

    pub fn mint(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.assert_seed(
            iter::once([seed, BlsScalar::from(TX_MINT as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .chain(iter::once(BlsScalar::from(value)))
                .collect(),
            signature,
        )?;

        self.assert_pause()?;
        self.assert_address(&address)?;

        self.total_supply = self
            .total_supply
            .checked_add(value)
            .ok_or(Error::BalanceOverflow)?;

        self.balance_add(&address, value)
    }

    pub fn burn(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        address: PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.assert_seed(
            iter::once([seed, BlsScalar::from(TX_BURN as u64)])
                .chain(iter::once(address.as_ref().to_hash_inputs()))
                .flatten()
                .chain(iter::once(BlsScalar::from(value)))
                .collect(),
            signature,
        )?;

        self.assert_pause()?;
        self.total_supply = self.total_supply.saturating_sub(value);

        self.balance_sub(&address, value);

        Ok(())
    }

    pub fn transfer(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        batch: Vec<Transfer>,
    ) -> Result<(), Error> {
        self.assert_seed(
            iter::once([seed, BlsScalar::from(TX_TRANSFER as u64)])
                .flatten()
                .chain(batch.iter().flat_map(Transfer::as_scalars))
                .collect(),
            signature,
        )?;
        self.assert_pause()?;

        for Transfer {
            from, to, amount, ..
        } in batch
        {
            self.assert_address(&from)?;

            let remaining = self.balance_sub(&from, amount);

            if remaining > 0 {
                self.mint(seed, signature, from, remaining)?
            }

            self.balance_add(&to, amount)?;
        }

        Ok(())
    }
}
