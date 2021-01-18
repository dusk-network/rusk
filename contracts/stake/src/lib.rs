// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]
#![warn(missing_docs)]

//! This module contains the logic for the staking contract, which is used to
//! maintain the provisioner committee.

mod counter;
mod key;
mod map;
mod set;
mod stake;

use canonical::{Canon, Sink, Source, Store};

pub use counter::Counter;
pub use key::Key;
pub use map::StakeMapping;
pub use set::IdentifierSet;
pub use stake::Stake;

/// This module contains all opcodes for the staking contract.
pub mod ops {
    // Queries
    /// Opcode for finding a specific stake in the contract.
    pub const FIND_STAKE: u16 = 0x00;

    // Transactions
    /// Opcode for adding a stake to the contract.
    pub const STAKE: u16 = 0x01;
    /// Opcode for extending an existing stake in the contract.
    pub const EXTEND_STAKE: u16 = 0x02;
    /// Opcode for retrieving an existing stake in the contract.
    pub const WITHDRAW_STAKE: u16 = 0x03;
    /// Opcode for punishing a malicious provisioner.
    pub const SLASH: u16 = 0x04;
}

/// The staking contract. It contains a mapping of a provisioner's public key to
/// his stake value and some extra info, as well as a set which contains all
/// provisioner public keys in order of being added to the contract. The
/// contract is responsible for maintaining the committee and allows users to
/// start staking, extend their stakes, and withdraw their stakes.
///
/// Note that rewards are distributed in a separate contract, this contract is
/// purely for management of stake lifetimes.
#[derive(Debug, Clone)]
pub struct Contract<S: Store> {
    stake_mapping: StakeMapping<S>,
    stake_identifier_set: IdentifierSet<S>,
    counter: Counter,
}

impl<S> Default for Contract<S>
where
    S: Store,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Canon<S> for Contract<S>
where
    S: Store,
{
    fn read(source: &mut impl Source<S>) -> Result<Self, S::Error> {
        Ok(Contract {
            stake_mapping: Canon::<S>::read(source)?,
            stake_identifier_set: Canon::<S>::read(source)?,
            counter: Canon::<S>::read(source)?,
        })
    }

    fn write(&self, sink: &mut impl Sink<S>) -> Result<(), S::Error> {
        self.stake_mapping.write(sink)?;
        self.stake_identifier_set.write(sink)?;
        self.counter.write(sink)
    }

    fn encoded_len(&self) -> usize {
        Canon::<S>::encoded_len(&self.stake_mapping)
            + Canon::<S>::encoded_len(&self.stake_identifier_set)
            + Canon::<S>::encoded_len(&self.counter)
    }
}

impl<S: Store> Contract<S> {
    /// Return a new, empty instance of the staking contract.
    pub fn new() -> Self {
        Self {
            stake_mapping: StakeMapping::new(),
            stake_identifier_set: IdentifierSet::new(),
            counter: Counter::default(),
        }
    }

    /// Returns a reference to the stake contract's mapping.
    pub fn stake_mapping(&self) -> &StakeMapping<S> {
        &self.stake_mapping
    }

    /// Returns a mutable reference to the stake contract's mapping.
    pub fn stake_mapping_mut(&mut self) -> &mut StakeMapping<S> {
        &mut self.stake_mapping
    }

    /// Returns a reference to the stake contract's identifier set.
    pub fn stake_identifier_set(&self) -> &IdentifierSet<S> {
        &self.stake_identifier_set
    }

    /// Returns a mutable reference to the stake contract's identifier set.
    pub fn stake_identifier_set_mut(&mut self) -> &mut IdentifierSet<S> {
        &mut self.stake_identifier_set
    }

    /// Returns a reference to the stake contract's counter.
    pub fn counter(&self) -> &Counter {
        &self.counter
    }

    /// Returns a mutable reference to the stake contract's counter.
    pub fn counter_mut(&mut self) -> &mut Counter {
        &mut self.counter
    }
}

#[cfg(feature = "host")]
pub mod host;

#[cfg(feature = "hosted")]
pub mod hosted;
