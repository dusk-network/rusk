// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey as BlsPublicKey, Signature};

#[cfg(not(target_arch = "wasm32"))]
use dusk_bls12_381_sign::APK as AggregatedBlsPublicKey;

use crate::collection::{Map, Set};
use crate::*;

#[derive(Debug, Clone, Default, Canon)]
pub struct GovernanceContract {
    pub(crate) seeds: Set<BlsScalar>,
    pub(crate) balances: Map<PublicKey, u64>,
    pub(crate) paused: bool,
    pub(crate) total_supply: u64,
    // The `broker` and the `authority` needs to be public so they can be
    // accessed from the host environment (rusk).
    // TODO: `broker` is wrapped in an option because does not support
    // `Default`
    pub broker: Option<PublicKey>,
    pub authority: BlsPublicKey,
}

/// Use `GovernanceContract::default()` for instance.
impl GovernanceContract {
    pub fn verify(
        &mut self,
        seed: BlsScalar,
        signature: Signature,
        message: &[u8],
    ) -> Result<(), Error> {
        self.assert_seed(seed)?;

        #[cfg(target_arch = "wasm32")]
        if !rusk_abi::verify_bls_sign(
            signature,
            self.authority,
            message.to_vec(),
        ) {
            return Err(Error::InvalidSignature);
        }

        #[cfg(not(target_arch = "wasm32"))]
        AggregatedBlsPublicKey::from(&self.authority)
            .verify(&signature, message)
            .or(Err(Error::InvalidSignature))?;
        Ok(())
    }
    /// Seed invariant: asserts that the seed is valid and is not already used
    fn assert_seed(&mut self, seed: BlsScalar) -> Result<(), Error> {
        if self.seeds.contains(&seed) {
            return Err(Error::SeedAlreadyUsed);
        }
        self.seeds.insert(seed);

        Ok(())
    }

    /// Running invariant: asserts the contract is running and not paused
    fn assert_running(&self) -> Result<(), Error> {
        if self.paused {
            Err(Error::ContractIsPaused)
        } else {
            Ok(())
        }
    }

    /// Add the value given to the specified address' balance.
    /// If the address is not present, it will be created with the given value
    /// as balance.
    ///
    /// Returns an error if the balance overflows.
    fn add_balance(
        &mut self,
        address: &PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        // No matter if the address exists or not, if the value is `0` we bail
        // out
        if value == 0 {
            return Ok(());
        }

        if let Some(balance) = self.balances.get_mut(address) {
            *balance =
                balance.checked_add(value).ok_or(Error::BalanceOverflow)?;
        } else {
            self.balances.insert(*address, value);
        }

        Ok(())
    }

    /// Subtract the value given from the specified address' balance.
    /// If the address is not present nothing happens.
    fn sub_balance(&mut self, address: &PublicKey, value: u64) -> u64 {
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
            None => value,
        }
    }

    pub fn balance(&self, address: &PublicKey) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.paused = false;
    }

    pub fn mint(
        &mut self,
        address: &PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.assert_running()?;

        self.total_supply = self
            .total_supply
            .checked_add(value)
            .ok_or(Error::BalanceOverflow)?;

        self.add_balance(address, value)
    }

    pub fn burn(
        &mut self,
        address: &PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        self.assert_running()?;

        let remaining = self.sub_balance(address, value);

        self.total_supply = self.total_supply.saturating_sub(value - remaining);

        Ok(())
    }

    fn checked_transfer(
        &mut self,
        from: &PublicKey,
        to: &PublicKey,
        amount: u64,
    ) -> Result<(), Error> {
        let remaining = self.sub_balance(from, amount);

        if remaining > 0 {
            self.mint(to, remaining)?
        }

        self.add_balance(to, amount - remaining)?;

        Ok(())
    }

    pub fn transfer(&mut self, batch: Vec<Transfer>) -> Result<(), Error> {
        self.assert_running()?;

        for Transfer {
            mut from,
            mut to,
            amount,
            ..
        } in batch
        {
            if from == self.broker {
                from.take();
            }

            if to == self.broker {
                to.take();
            }

            match (from, to) {
                (None, None) => {}
                // Withdraw or Transfer to the `broker`
                (Some(from), None) => {
                    self.burn(&from, amount)?;
                }
                // Deposit or Transfer from the `broker`
                (None, Some(to)) => {
                    self.mint(&to, amount)?;
                }
                // Transfer between two shareholders
                (Some(from), Some(to)) => {
                    self.checked_transfer(&from, &to, amount)?;
                }
            }
        }

        Ok(())
    }

    pub fn fee(&mut self, batch: Vec<Transfer>) -> Result<(), Error> {
        self.assert_running()?;

        for Transfer { from, amount, .. } in batch {
            if let (Some(from), Some(broker)) = (from, self.broker) {
                self.checked_transfer(&from, &broker, amount)?;
            }
        }

        Ok(())
    }
}
