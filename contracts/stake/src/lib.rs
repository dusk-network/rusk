// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use canonical_derive::Canon;
use dusk_abi::ContractId;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use dusk_pki::PublicKey;
use rusk_abi::PaymentInfo;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub const EPOCH: u32 = 2160;
pub const MATURITY: u32 = 2 * EPOCH;
pub const VALIDITY: u32 = 56 * EPOCH;
pub const MINIMUM_STAKE: u64 = 10_000;
pub const MAXIMUM_STAKE: u64 = 1_000_000;
pub const PAYMENT_INFO: PaymentInfo = PaymentInfo::Transparent(None);

pub type PublicKeyBytes = [u8; PublicKey::SIZE];

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
    staked: Map<PublicKeyBytes, Stake>,
}

impl StakeContract {
    pub fn transfer_contract() -> ContractId {
        ContractId::from([
            0xd3, 0xf8, 0x7f, 0xfc, 0x1b, 0xc7, 0x43, 0x1d, 0xde, 0x81, 0x5f, 0xb1, 0xe1, 0x1b,
            0xd0, 0xfe, 0x88, 0x37, 0x1a, 0x15, 0x4a, 0xec, 0x27, 0x5d, 0xed, 0x2, 0x4d, 0x8c,
            0xc0, 0xf7, 0x99, 0x5f,
        ])
    }
}
