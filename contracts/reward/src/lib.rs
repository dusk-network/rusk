// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]

mod key;
mod map;
mod publickeys;

use canonical::{Canon, Sink, Source, Store};

pub use key::Key;
pub use map::BalanceMapping;
pub use publickeys::PublicKeys;

mod ops {
    // Queries
    pub const GET_BALANCE: u16 = 0x00;
    pub const GET_WITHDRAWAL_TIME: u16 = 0x01;

    // Transactions
    pub const DISTRIBUTE: u16 = 0x02;
    pub const WITHDRAW: u16 = 0x03;
}

#[derive(Debug, Clone)]
pub struct Contract<S: Store> {
    balance_mapping: BalanceMapping<S>,
}

impl<S> Canon<S> for Contract<S>
where
    S: Store,
{
    fn read(source: &mut impl Source<S>) -> Result<Self, S::Error> {
        Ok(Contract {
            balance_mapping: Canon::<S>::read(source)?,
        })
    }

    fn write(&self, sink: &mut impl Sink<S>) -> Result<(), S::Error> {
        self.balance_mapping.write(sink)
    }

    fn encoded_len(&self) -> usize {
        Canon::<S>::encoded_len(&self.balance_mapping)
    }
}

impl<S: Store> Contract<S> {
    pub fn new() -> Self {
        Self {
            balance_mapping: BalanceMapping::new(),
        }
    }

    pub fn balance_mapping(&self) -> &BalanceMapping<S> {
        &self.balance_mapping
    }

    pub fn balance_mapping_mut(&mut self) -> &mut BalanceMapping<S> {
        &mut self.balance_mapping
    }
}

#[cfg(feature = "host")]
pub mod host;

#[cfg(feature = "hosted")]
pub mod hosted;
