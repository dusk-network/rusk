// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bls12_381_bls::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable;
use jubjub_schnorr::PublicKey as SchnorrPublicKey;
use phoenix_core::PublicKey;
use rusk_abi::ContractId;
use serde_derive::{Deserialize, Serialize};

use super::wrapper::Wrapper;
use crate::state;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct Governance {
    pub(crate) contract_owner: Option<Wrapper<PublicKey, { PublicKey::SIZE }>>,
    pub contract_address: u64,
    pub name: String,
    pub(crate) authority: Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>,
    pub(crate) broker: Wrapper<SchnorrPublicKey, { SchnorrPublicKey::SIZE }>,
}

impl Governance {
    pub fn owner(&self) -> [u8; PublicKey::SIZE] {
        let dusk = Wrapper::from(*state::DUSK_KEY);
        self.contract_owner.as_ref().unwrap_or(&dusk).to_bytes()
    }

    pub fn authority(&self) -> &BlsPublicKey {
        &self.authority
    }
    pub fn broker(&self) -> &SchnorrPublicKey {
        &self.broker
    }

    pub fn contract(&self) -> ContractId {
        let mut data = [0u8; 32];
        let address = self.contract_address.to_be_bytes();
        data[24..].copy_from_slice(&address);
        ContractId::from(data)
    }
}
