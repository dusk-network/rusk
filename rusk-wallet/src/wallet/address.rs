// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::hash::Hasher;
use std::{fmt, str::FromStr};

use super::*;
use crate::Error;

use dusk_bytes::{DeserializableSlice, Serializable};

/// Address to perform a transaction with.
#[derive(Clone, Eq)]
#[allow(missing_docs)]
pub enum Address {
    /// Shielded address for shielded transactions.
    Shielded { addr: PhoenixPublicKey },
    /// Public account address for public transactions and staking
    /// operations.
    Public { addr: BlsPublicKey },
}

impl Address {
    /// Returns the inner shielded address, if present.
    ///
    /// # Errors
    /// If the inner address is a public one.
    pub fn shielded_address(&self) -> Result<&PhoenixPublicKey, Error> {
        if let Self::Shielded { addr } = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedPhoenixPublicKey)
        }
    }

    /// Returns the inner public address, if present.
    ///
    /// # Errors
    /// If the inner address is a shielded one.
    pub fn public_address(&self) -> Result<&BlsPublicKey, Error> {
        if let Self::Public { addr } = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedBlsPublicKey)
        }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Shielded { addr } => addr.to_bytes().to_vec(),
            Self::Public { addr } => addr.to_bytes().to_vec(),
        }
    }

    /// A trimmed version of the address to display as preview
    pub fn preview(&self) -> String {
        let addr_key_str = String::from(self);
        format!(
            "{}...{}",
            &addr_key_str[..5],
            &addr_key_str[addr_key_str.len() - 5..]
        )
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address_bytes = bs58::decode(s).into_vec()?;
        let mut address_reader = &address_bytes[..];

        match address_bytes.len() {
            PhoenixPublicKey::SIZE => Ok(Self::Shielded {
                addr: PhoenixPublicKey::from_reader(&mut address_reader)
                    .map_err(Error::Bytes)?,
            }),
            BlsPublicKey::SIZE => Ok(Self::Public {
                addr: BlsPublicKey::from_reader(&mut address_reader)
                    .map_err(Error::Bytes)?,
            }),
            _ => Err(Error::Bytes(dusk_bytes::Error::InvalidData)),
        }
    }
}

impl From<&Address> for String {
    fn from(address: &Address) -> Self {
        match address {
            Address::Shielded { addr } => {
                bs58::encode(addr.to_bytes()).into_string()
            }
            Address::Public { addr } => {
                bs58::encode(addr.to_bytes()).into_string()
            }
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Address::Shielded { addr: self_pk },
                Address::Shielded { addr: other_pk },
            ) => self_pk == other_pk,
            (
                Address::Public { addr: self_pk },
                Address::Public { addr: other_pk },
            ) => self_pk == other_pk,
            _ => false,
        }
    }
}

impl std::hash::Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_bytes().hash(state);
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

/// Profile struct containing the addresses used for shielded and public
/// transactions as well as for staking operations.
pub struct Profile {
    /// Shielded address for shielded transactions
    pub shielded_addr: PhoenixPublicKey,
    /// Public account address for public transactions and staking operations.
    pub public_addr: BlsPublicKey,
}

impl Profile {
    /// Format the shielded address into a string.
    pub fn shielded_address_string(&self) -> String {
        format!(
            "{} - {}",
            shielded_address_prefix(),
            Address::Shielded {
                addr: self.shielded_addr,
            }
        )
    }

    /// Format the public account into a string.
    pub fn public_account_string(&self) -> String {
        format!(
            "{} - {}",
            public_account_prefix(),
            Address::Public {
                addr: self.public_addr
            }
        )
    }

    /// Format the staking account into a string.
    pub fn staking_account_string(&self) -> String {
        format!(
            "{} - {}",
            staking_account_prefix(),
            Address::Public {
                addr: self.public_addr
            }
        )
    }

    /// Format the shortened shielded address into a string.
    pub fn shielded_address_preview(&self) -> String {
        format!(
            "{} - {}",
            shielded_address_prefix(),
            Address::Shielded {
                addr: self.shielded_addr,
            }
            .preview(),
        )
    }

    /// Format the shortened public account into a string.
    pub fn public_account_preview(&self) -> String {
        format!(
            "{} - {}",
            public_account_prefix(),
            Address::Public {
                addr: self.public_addr
            }
            .preview()
        )
    }

    /// Format the shortened staking account into a string.
    pub fn staking_account_preview(&self) -> String {
        format!(
            "{} - {}",
            staking_account_prefix(),
            Address::Public {
                addr: self.public_addr
            }
            .preview()
        )
    }

    /// Format the profile's index.
    pub fn index_string(profile_idx: u8) -> String {
        let mut index_string = format!("Profile {:2}", profile_idx + 1);
        if profile_idx == 0 {
            index_string.push_str(" (Default)");
        }

        index_string
    }
}

fn shielded_address_prefix() -> String {
    format!("{:<16}", "Shielded address")
}

fn public_account_prefix() -> String {
    format!("{:<16}", "Public account")
}

fn staking_account_prefix() -> String {
    format!("{:<16}", "Staking account")
}
