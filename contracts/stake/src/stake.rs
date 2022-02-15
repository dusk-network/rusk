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
    eligibility: BlockHeight,
    created_at: BlockHeight,
}

impl Stake {
    pub const fn new(
        value: u64,
        created_at: BlockHeight,
        block_height: BlockHeight,
    ) -> Self {
        let epoch = Self::epoch(block_height);
        let eligibility = block_height + MATURITY + epoch;

        Self::with_eligibility(value, created_at, eligibility)
    }

    pub const fn with_eligibility(
        value: u64,
        created_at: BlockHeight,
        eligibility: BlockHeight,
    ) -> Self {
        Self {
            value,
            created_at,
            eligibility,
        }
    }

    pub const fn epoch(block_height: BlockHeight) -> u64 {
        EPOCH - block_height % EPOCH
    }

    pub const fn value(&self) -> u64 {
        self.value
    }

    pub const fn eligibility(&self) -> BlockHeight {
        self.eligibility
    }

    pub const fn created_at(&self) -> BlockHeight {
        self.created_at
    }

    pub const fn is_valid(&self, block_height: u64) -> bool {
        self.eligibility <= block_height
    }
}

impl Serializable<24> for Stake {
    type Error = Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut value = [0u8; 8];
        let mut eligibility = [0u8; 8];
        let mut created_at = [0u8; 8];

        value.copy_from_slice(&buf[..8]);
        eligibility.copy_from_slice(&buf[8..16]);
        created_at.copy_from_slice(&buf[16..24]);

        let value = u64::from_le_bytes(value);
        let eligibility = u64::from_le_bytes(eligibility);
        let created_at = BlockHeight::from_le_bytes(created_at);

        Ok(Self {
            value,
            eligibility,
            created_at,
        })
    }

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];

        (&mut bytes[..8]).copy_from_slice(&self.value.to_le_bytes());
        (&mut bytes[8..16]).copy_from_slice(&self.eligibility.to_le_bytes());
        (&mut bytes[16..24]).copy_from_slice(&self.created_at.to_le_bytes());

        bytes
    }
}
