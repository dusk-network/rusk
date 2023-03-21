// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable;
use dusk_pki::PublicKey;
use rusk_abi::ModuleId;
use serde_derive::{Deserialize, Serialize};

use super::wrapper::Wrapper;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct Governance {
    pub contract_address: u64,
    pub name: String,
    pub(crate) authority: Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>,
    pub(crate) broker: Wrapper<PublicKey, { PublicKey::SIZE }>,
}

impl Governance {
    pub fn authority(&self) -> &BlsPublicKey {
        &self.authority
    }
    pub fn broker(&self) -> &PublicKey {
        &self.broker
    }

    pub fn contract(&self) -> ModuleId {
        let mut data = [0u8; 32];
        let address = self.contract_address.to_be_bytes();
        data[24..].copy_from_slice(&address);
        ModuleId::from(data)
    }
}
