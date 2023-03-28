// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use alloc::boxed::Box;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey as BlsPublicKey, Signature};
use dusk_pki::PublicKey;

use crate::collection::{Map, Set};
use crate::Transfer;

#[derive(Debug)]
pub struct GovernanceState {
    balances: Map<PublicKey, u64>,
    seeds: Set<BlsScalar>,

    total_supply: u64,
    paused: bool,

    broker: AtomicPtr<PublicKey>,
    authority: AtomicPtr<BlsPublicKey>,
}

impl GovernanceState {
    /// Create a new instance of the governance state.
    pub const fn new() -> Self {
        Self {
            balances: Map::new(),
            seeds: Set::new(),
            total_supply: 0,
            paused: false,
            broker: AtomicPtr::new(ptr::null_mut()),
            authority: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Executes a `batch` of transfers.
    pub fn transfer(&mut self, batch: Vec<Transfer>) {
        if self.paused {
            panic!("The contract is paused");
        }

        let broker = self.get_broker();

        for (mut from, mut to, amount, _) in batch {
            if let Some(f) = &from {
                if f == &broker {
                    from.take();
                }
            }
            if let Some(t) = &to {
                if t == &broker {
                    to.take();
                }
            }

            match (from, to) {
                (None, None) => {}
                // Withdraw or Transfer to the `broker`
                (Some(from), None) => {
                    self.burn(from, amount);
                }
                // Deposit or Transfer from the `broker`
                (None, Some(to)) => {
                    self.mint(to, amount);
                }
                // Transfer between two shareholders
                (Some(from), Some(to)) => {
                    self.checked_transfer(from, to, amount);
                }
            }
        }
    }

    /// Pays fees to the broker.
    pub fn fee(&mut self, batch: Vec<Transfer>) {
        if self.paused {
            panic!("The contract is paused");
        }

        let broker = self.get_broker();

        for (from, _, amount, _) in batch {
            if let Some(from) = from {
                self.checked_transfer(from, broker, amount);
            }
        }
    }

    /// Mints a given `amount` of tokens to the given `address`.
    pub fn mint(&mut self, address: PublicKey, amount: u64) {
        if self.paused {
            panic!("The contract is paused");
        }

        let new_supply = self.total_supply.checked_add(amount);
        if new_supply.is_none() {
            panic!("Total supply overflow");
        }
        self.total_supply = new_supply.unwrap();

        self.add_balance(address, amount);
    }

    /// Burns a given `amount` of tokens from the given `address`.
    pub fn burn(&mut self, address: PublicKey, amount: u64) {
        if self.paused {
            panic!("The contract is paused");
        }

        let remaining = self.sub_balance(address, amount);

        self.total_supply =
            self.total_supply.saturating_sub(amount - remaining);
    }

    /// Pause the governance contract.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpause the governance contract.
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Set the broker in the contract.
    ///
    /// This function should only be called once, but it does support being
    /// called multiple times.
    pub fn set_broker(&self, broker: PublicKey) {
        let broker = Box::leak(Box::new(broker));

        let last_broker =
            self.broker.swap(broker as *mut PublicKey, Ordering::SeqCst);

        // If the broker was already set we need to recoup the memory, otherwise
        // we will leak memory.
        if last_broker != ptr::null_mut() {
            unsafe { Box::from_raw(last_broker) };
        }
    }

    /// Set the authority in the contract.
    ///
    /// This function should only be called once, but it does support being
    /// called multiple times.
    pub fn set_authority(&self, authority: BlsPublicKey) {
        let authority = Box::leak(Box::new(authority));
        let last_authority = self
            .authority
            .swap(authority as *mut BlsPublicKey, Ordering::SeqCst);

        // If the authority was already set we need to recoup the memory,
        // otherwise we will leak memory.
        if last_authority != ptr::null_mut() {
            unsafe { Box::from_raw(last_authority) };
        }
    }

    /// Asserts that the signature and the given seed are valid. The seed
    /// shouldn't have been used before.
    ///
    /// # Panics
    /// When the signature isn't valid, the seed has already been used, or the
    /// authority hasn't been set.
    pub fn assert_signature(
        &mut self,
        signature: Signature,
        seed: BlsScalar,
        message: Vec<u8>,
    ) {
        let authority = self.get_authority();

        if self.seeds.contains(&seed) {
            panic!("Seed already used");
        }
        self.seeds.insert(seed);

        if !rusk_abi::verify_bls(message, authority, signature) {
            panic!("Invalid signature");
        }
    }

    /// Get the current authority.
    ///
    /// # Panics
    /// If the authority hasn't been set.
    pub fn get_authority(&self) -> BlsPublicKey {
        let authority = self.authority.load(Ordering::SeqCst);
        if authority.is_null() {
            panic!("Authority not set");
        }
        unsafe { *authority }
    }

    /// Get the current broker.
    ///
    /// # Panics
    /// If the broker hasn't been set.
    pub fn get_broker(&self) -> PublicKey {
        let broker = self.broker.load(Ordering::SeqCst);
        if broker.is_null() {
            panic!("Broker not set");
        }
        unsafe { *broker }
    }

    /// Add the value given to the specified address' balance.
    /// If the address is not present, it will be created with the given value
    /// as balance.
    ///
    /// # Panics
    /// If the balance overflows.
    fn add_balance(&mut self, address: PublicKey, amount: u64) {
        // Return early if the amount is zero
        if amount == 0 {
            return;
        }

        if let Some(balance) = self.balances.get_mut(&address) {
            let new_balance = balance.checked_add(amount);
            if new_balance.is_none() {
                panic!("Balance overflow");
            }
            *balance = new_balance.unwrap();
        } else {
            self.balances.insert(address, amount);
        }
    }

    /// Subtract the value given from the specified address' balance, returning
    /// the remainder of the subtraction. If the address is not present nothing
    /// happens.
    fn sub_balance(&mut self, address: PublicKey, value: u64) -> u64 {
        match self.balances.get_mut(&address) {
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
        self.balances.get(address).unwrap_or(&0).clone()
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    fn checked_transfer(
        &mut self,
        from: PublicKey,
        to: PublicKey,
        amount: u64,
    ) {
        let remaining = self.sub_balance(from, amount);

        if remaining > 0 {
            self.mint(to, remaining);
        }

        self.add_balance(to, amount - remaining);
    }
}
