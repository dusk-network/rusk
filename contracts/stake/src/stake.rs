// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical_derive::Canon;
use dusk_bytes::Serializable;

#[derive(Debug, Default, Clone, Copy, Canon, PartialEq, Eq)]
pub struct Stake {
    value: u64,
    eligibility: u64,
    expiration: u64,
}

impl Stake {
    pub const fn new(value: u64, eligibility: u64, expiration: u64) -> Self {
        Self {
            value,
            eligibility,
            expiration,
        }
    }

    pub const fn from_block_height(value: u64, block_height: u64) -> Stake {
        let epoch = Self::epoch(block_height);
        let eligibility = block_height + MATURITY + epoch;
        let expiration = block_height + MATURITY + VALIDITY + epoch;

        Self::new(value, eligibility, expiration)
    }

    pub const fn epoch(block_height: u64) -> u64 {
        EPOCH - block_height % EPOCH
    }

    pub const fn value(&self) -> u64 {
        self.value
    }

    pub const fn eligibility(&self) -> u64 {
        self.eligibility
    }

    pub const fn expiration(&self) -> u64 {
        self.expiration
    }

    pub const fn is_valid(&self, block_height: u64) -> bool {
        self.eligibility <= block_height && block_height < self.expiration
    }

    pub fn extend(&mut self) {
        self.expiration += VALIDITY;
    }
}

impl Serializable<24> for Stake {
    type Error = Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut value = [0u8; 8];
        let mut eligibility = [0u8; 8];
        let mut expiration = [0u8; 8];

        value.copy_from_slice(&buf[..8]);
        eligibility.copy_from_slice(&buf[8..16]);
        expiration.copy_from_slice(&buf[16..24]);

        let value = u64::from_le_bytes(value);
        let eligibility = u64::from_le_bytes(eligibility);
        let expiration = u64::from_le_bytes(expiration);

        let stake = Self::new(value, eligibility, expiration);

        Ok(stake)
    }

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];

        (&mut bytes[..8]).copy_from_slice(&self.value.to_le_bytes());
        (&mut bytes[8..16]).copy_from_slice(&self.eligibility.to_le_bytes());
        (&mut bytes[16..24]).copy_from_slice(&self.expiration.to_le_bytes());

        bytes
    }
}
