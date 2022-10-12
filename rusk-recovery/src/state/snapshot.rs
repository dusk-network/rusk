// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use rusk_abi::dusk::Dusk;
use serde_derive::{Deserialize, Serialize};

mod acl;
mod stake;
mod wrapper;

use acl::Acl;
pub use stake::GenesisStake;
use wrapper::Wrapper;

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
    acl: Acl,

    // This "serde skip" workaround seems needed as per https://github.com/toml-rs/toml-rs/issues/384
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    balance: Vec<Balance>,
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
    /// Returns an iterator of the transfers included in this snapshot
    pub fn transfers(&self) -> impl Iterator<Item = &Balance> {
        self.balance.iter()
    }

    /// Returns an iterator of the stakes included in this snapshot.
    pub fn stakes(&self) -> impl Iterator<Item = &GenesisStake> {
        self.stake.iter()
    }

    /// Returns an iterator of the owners of the staking contract
    pub fn owners(&self) -> impl Iterator<Item = &PublicKey> {
        self.acl.stake.owners.iter().map(|pk| &**pk)
    }

    /// Returns an iterator of the allowed staking addresses
    pub fn allowlist(&self) -> impl Iterator<Item = &PublicKey> {
        self.acl.stake.allowlist.iter().map(|pk| &**pk)
    }

    pub fn base_state(&self) -> Option<&str> {
        self.base_state.as_deref()
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    use super::{acl::Users, *};

    use crate::{
        provisioners,
        state::{self, Balance, GenesisStake},
    };
    use rusk_abi::dusk::{dusk, Dusk};
    use stake_contract::MINIMUM_STAKE;

    /// Amount of the note inserted in the genesis state.
    const GENESIS_DUSK: Dusk = dusk(1_000.0);

    /// Faucet note value.
    const FAUCET_DUSK: Dusk = dusk(500_000_000.0);

    fn localnet_snapshot() -> Snapshot {
        let users = provisioners::keys(false);
        let owners = users.iter().map(|&p| p.into()).collect();
        let allowlist = users.iter().map(|&p| p.into()).collect();
        let stake = users
            .iter()
            .map(|&p| GenesisStake {
                address: p.into(),
                amount: MINIMUM_STAKE,
                eligibility: None,
                reward: None,
            })
            .collect();

        Snapshot {
            base_state: None,
            balance: vec![Balance {
                address: (*state::DUSK_KEY).into(),
                seed: Some(0xdead_beef),
                notes: vec![GENESIS_DUSK],
            }],
            stake,
            acl: Acl {
                stake: Users { allowlist, owners },
            },
        }
    }

    fn testnet_snapshot() -> Snapshot {
        let seed = Some(0xdead_beef);
        let users = provisioners::keys(true);
        let allowlist = users.iter().map(|&p| p.into()).collect();
        let owners = vec![(*provisioners::DUSK_KEY).into()];
        let stake = users
            .iter()
            .map(|&p| GenesisStake {
                address: p.into(),
                amount: dusk(2_000_000.0),
                eligibility: None,
                reward: None,
            })
            .collect();

        Snapshot {
            base_state: Some("https://dusk-infra.ams3.digitaloceanspaces.com/keys/genesis.zip".into()),
            balance: vec![
                Balance {
                    address: (*state::DUSK_KEY).into(),
                    seed,
                    notes: vec![GENESIS_DUSK],
                },
                Balance {
                    address: (*state::FAUCET_KEY).into(),
                    seed,
                    notes: vec![FAUCET_DUSK],
                },
            ],
            stake,
            acl: Acl {
                stake: Users { allowlist, owners },
            },
        }
    }

    /// Returns a Snapshot compliant with the old hardcode localnet.
    ///
    /// This will be removed in a future version when will be possible to pass a
    /// configuration file to rusk-recovery-state
    pub(crate) fn localnet_from_file() -> Result<Snapshot, Box<dyn Error>> {
        let toml = include_str!("../../config/localnet.toml");
        let snapshot = toml::from_str(toml)?;
        Ok(snapshot)
    }

    /// Returns a Snapshot compliant with the old hardcode testnet.
    ///
    /// This will be removed in a future version when will be possible to pass a
    /// configuration file to rusk-recovery-state
    pub(crate) fn testnet_from_file() -> Result<Snapshot, Box<dyn Error>> {
        let toml = include_str!("../../config/testnet.toml");
        let snapshot = toml::from_str(toml)?;
        Ok(snapshot)
    }

    #[test]
    fn testnet_toml() -> Result<(), Box<dyn Error>> {
        let testnet = testnet_snapshot();
        let str = toml::to_string_pretty(&testnet)?;

        let back: Snapshot = toml::from_str(&str)?;
        assert!(testnet == back);

        assert!(testnet == testnet_from_file()?);
        Ok(())
    }

    #[test]
    fn localnet_toml() -> Result<(), Box<dyn Error>> {
        let localnet = localnet_snapshot();
        let str = toml::to_string_pretty(&localnet)?;
        println!("{str}");

        let back: Snapshot = toml::from_str(&str)?;
        assert!(localnet == back);

        assert!(localnet == localnet_from_file()?);
        Ok(())
    }

    #[test]
    fn empty_toml() -> Result<(), Box<dyn Error>> {
        let str = toml::to_string_pretty(&Snapshot::default())?;
        let deserialized: Snapshot = toml::from_str(&str)?;

        // `Snapshot` is too big to be compared with assert_eq
        assert!(
            Snapshot::default() == deserialized,
            "Deserialized struct differs from the serialized one"
        );
        Ok(())
    }
}
