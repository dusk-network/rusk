// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::StakeKeys;
use dusk_core::Dusk;
use serde_derive::{Deserialize, Serialize};

use super::wrapper::Wrapper;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisStake {
    address: Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>,
    owner: Option<Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>>,
    pub amount: Dusk,
    pub eligibility: Option<u64>,
    pub reward: Option<Dusk>,
}

impl GenesisStake {
    pub fn address(&self) -> &BlsPublicKey {
        &self.address
    }

    pub fn to_stake_keys(&self) -> StakeKeys {
        let owner: &BlsPublicKey = self.owner.as_ref().unwrap_or(&self.address);
        StakeKeys::new(*self.address(), *owner)
    }
}
