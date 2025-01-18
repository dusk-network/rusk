// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::phoenix::PublicKey as PhoenixPublicKey;
use dusk_core::Dusk;
use serde_derive::{Deserialize, Serialize};

mod stake;
pub use stake::GenesisStake;
mod wrapper;
use wrapper::Wrapper;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct PhoenixBalance {
    address: Wrapper<PhoenixPublicKey, { PhoenixPublicKey::SIZE }>,
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub notes: Vec<Dusk>,
}

impl PhoenixBalance {
    pub fn address(&self) -> &PhoenixPublicKey {
        &self.address
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct MoonlightAccount {
    address: Wrapper<AccountPublicKey, { AccountPublicKey::SIZE }>,
    pub balance: Dusk,
}

impl MoonlightAccount {
    pub fn address(&self) -> &AccountPublicKey {
        &self.address
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Snapshot {
    base_state: Option<String>,
    owner: Option<Wrapper<AccountPublicKey, { AccountPublicKey::SIZE }>>,

    // This "serde skip" workaround seems needed as per https://github.com/toml-rs/toml-rs/issues/384
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    phoenix_balance: Vec<PhoenixBalance>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    moonlight_account: Vec<MoonlightAccount>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    stake: Vec<GenesisStake>,
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
    /// Returns an iterator over the phoenix balances included in this snapshot
    pub fn phoenix_balances(&self) -> impl Iterator<Item = &PhoenixBalance> {
        self.phoenix_balance.iter()
    }

    /// Returns an iterator of the moonlight accounts included in this snapshot
    pub fn moonlight_accounts(
        &self,
    ) -> impl Iterator<Item = &MoonlightAccount> {
        self.moonlight_account.iter()
    }

    /// Returns an iterator of the stakes included in this snapshot.
    pub fn stakes(&self) -> impl Iterator<Item = &GenesisStake> {
        self.stake.iter()
    }

    /// Return the owner of the smart contract.
    pub fn owner_or(
        &self,
        default: AccountPublicKey,
    ) -> [u8; AccountPublicKey::SIZE] {
        let default = Wrapper::from(default);
        self.owner.as_ref().unwrap_or(&default).to_bytes()
    }

    pub fn base_state(&self) -> Option<&str> {
        self.base_state.as_deref()
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    use dusk_bytes::DeserializableSlice;
    use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
    use dusk_core::stake::DEFAULT_MINIMUM_STAKE;
    use dusk_core::transfer::phoenix::PublicKey;

    use super::*;

    fn testnet_faucet_phoenix() -> PublicKey {
        let addr = include_str!("../../assets/faucet.address");
        let bytes = bs58::decode(addr).into_vec().expect("valid bs58");
        PublicKey::from_slice(&bytes).expect("faucet should have a valid key")
    }

    fn testnet_faucet_moonlight() -> AccountPublicKey {
        let addr = include_str!("../../assets/faucet.moonlight.address");
        let bytes = bs58::decode(addr).into_vec().expect("valid bs58");
        AccountPublicKey::from_slice(&bytes)
            .expect("faucet should have a valid key")
    }

    fn testnet_from_file() -> Result<Snapshot, Box<dyn Error>> {
        let toml = include_str!("../../config/testnet.toml");
        let snapshot = toml::from_str(toml)?;
        Ok(snapshot)
    }

    #[test]
    fn testnet_toml() -> Result<(), Box<dyn Error>> {
        let testnet = testnet_from_file()?;

        let faucet_phoenix = testnet
            .phoenix_balance
            .iter()
            .any(|b| b.address().eq(&testnet_faucet_phoenix()));

        let faucet_moonlight = testnet
            .moonlight_account
            .iter()
            .any(|b| b.address().eq(&testnet_faucet_moonlight()));

        if !faucet_phoenix && !faucet_moonlight {
            panic!("Testnet must have faucet configured");
        }

        if !testnet.stakes().any(|s| {
            s.amount >= DEFAULT_MINIMUM_STAKE
                && s.eligibility.unwrap_or_default() == 0
        }) {
            panic!("Testnet must have at least a provisioner configured");
        }

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
