// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use serde_derive::{Deserialize, Serialize};

use execution_core::StakePublicKey;
use rusk_abi::dusk::Dusk;

use super::wrapper::Wrapper;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisStake {
    pub(crate) address: Wrapper<StakePublicKey, { StakePublicKey::SIZE }>,
    pub amount: Dusk,
    pub eligibility: Option<u64>,
    pub reward: Option<Dusk>,
}

impl GenesisStake {
    pub fn address(&self) -> &StakePublicKey {
        &self.address
    }
}
