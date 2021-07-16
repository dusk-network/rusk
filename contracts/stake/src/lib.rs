// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use canonical_derive::Canon;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use dusk_pki::PublicKey;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub const MINIMUM_STAKE: u64 = 100_000_000_000_000;
pub const MAXIMUM_STAKE: u64 = 10_000_000_000_000_000;
pub const SLASH_REWARD: u64 = 50_000_000_000_000;
pub const ARBITRATION_MAX_HEIGHT: u64 = 6_311_520;

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
        self.expiration += rusk_abi::VALIDITY;
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct StakeContract {
    staked: Map<Key, Stake>,
}
