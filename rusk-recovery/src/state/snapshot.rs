// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use rusk_abi::dusk::Dusk;
use serde_derive::{Deserialize, Serialize};

mod governance;
mod stake;
mod wrapper;

use crate::state;
pub use stake::GenesisStake;
use wrapper::Wrapper;

pub use self::governance::Governance;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct Balance {
    address: Wrapper<PublicSpendKey, { PublicSpendKey::SIZE }>,
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub notes: Vec<Dusk>,
}

impl Balance {
    pub fn address(&self) -> &PublicSpendKey {
        &self.address
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Snapshot {
    base_state: Option<String>,
    owner: Option<Wrapper<PublicSpendKey, { PublicSpendKey::SIZE }>>,

    // This "serde skip" workaround seems needed as per https://github.com/toml-rs/toml-rs/issues/384
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    balance: Vec<Balance>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    stake: Vec<GenesisStake>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    governance: Vec<Governance>,
}

impl Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let toml = toml::to_string(self).map_err(|e| {
            let _ = writeln!(f, "{e}");
            std::fmt::Error
        })?;
        f.write_str(&toml)
    }
}

impl Snapshot {
    /// Returns an iterator of the transfers included in this snapshot
    pub fn transfers(&self) -> impl Iterator<Item = &Balance> {
        self.balance.iter()
    }

    /// Returns an iterator of the stakes included in this snapshot.
    pub fn stakes(&self) -> impl Iterator<Item = &GenesisStake> {
        self.stake.iter()
    }

    /// Return the owner of the smart contract.
    pub fn owner(&self) -> [u8; PublicSpendKey::SIZE] {
        let dusk = Wrapper::from(*state::DUSK_KEY);
        self.owner.as_ref().unwrap_or(&dusk).to_bytes()
    }

    pub fn base_state(&self) -> Option<&str> {
        self.base_state.as_deref()
    }

    pub fn governance_contracts(&self) -> impl Iterator<Item = &Governance> {
        self.governance.iter()
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    use super::*;

    use crate::state;

    pub(crate) fn testnet_from_file() -> Result<Snapshot, Box<dyn Error>> {
        let toml = include_str!("../../config/testnet.toml");
        let snapshot = toml::from_str(toml)?;
        Ok(snapshot)
    }

    #[test]
    fn testnet_toml() -> Result<(), Box<dyn Error>> {
        let testnet = testnet_from_file()?;

        testnet
            .balance
            .iter()
            .find(|b| b.address().eq(&*state::FAUCET_KEY))
            .expect("Testnet must have faucet configured");

        testnet
            .stakes()
            .next()
            .expect("Testnet must have at least a provisioner configured");

        Ok(())
    }

    #[test]
    fn empty_toml() -> Result<(), Box<dyn Error>> {
        let str = toml::to_string_pretty(&Snapshot::default())?;
        let deserialized: Snapshot = toml::from_str(&str)?;

        // `Snapshot` is too big to be compared with assert_eq
        assert_eq!(
            Snapshot::default(),
            deserialized,
            "Deserialized struct differs from the serialized one"
        );
        Ok(())
    }
}
