// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use canonical::CanonError;
use canonical_derive::Canon;
use core::ops::Deref;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use dusk_pki::PublicKey;

#[cfg(target_arch = "wasm32")]
mod wasm;

/// Epoch used for stake operations
pub const EPOCH: u32 = 2160;

/// Maturity of the stake
pub const MATURITY: u32 = 2 * EPOCH;

/// Validity of the stake
pub const VALIDITY: u32 = 56 * EPOCH;

/// The minimum amount of Dusk one can stake.
pub const MINIMUM_STAKE: u64 = 5_000;

pub type Key = [u8; PublicKey::SIZE];

#[derive(Debug, Default, Clone, Copy, Canon)]
pub struct Stake {
    value: u64,
    eligibility: u32,
    expiration: u32,
}

impl Stake {
    pub const fn new(value: u64, eligibility: u32, expiration: u32) -> Self {
        Self {
            value,
            eligibility,
            expiration,
        }
    }

    pub const fn value(&self) -> u64 {
        self.value
    }

    pub const fn eligibility(&self) -> u32 {
        self.eligibility
    }

    pub const fn expiration(&self) -> u32 {
        self.expiration
    }

    pub fn extend(&mut self) {
        self.expiration += VALIDITY;
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct StakeContract {
    pub staked: Map<Key, Stake>,
}
