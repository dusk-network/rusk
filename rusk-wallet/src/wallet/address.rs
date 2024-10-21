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
    Shielded(PhoenixPublicKey),
    /// Public account address for public transactions and staking
    /// operations.
    Public(BlsPublicKey),
}

impl Address {
    /// Check if the `other` Address uses the same transaction model
    pub fn same_transaction_model(&self, other: &Address) -> Result<(), Error> {
        match (self, other) {
            (Address::Shielded(_), Address::Shielded(_)) => Ok(()),
            (Address::Public(_), Address::Public(_)) => Ok(()),
            _ => Err(Error::DifferentTransactionModels),
        }
    }

    /// Returns the inner shielded key, if present.
    ///
    /// # Errors
    /// If the address is a public one.
    pub fn shielded_key(&self) -> Result<&PhoenixPublicKey, Error> {
        if let Self::Shielded(addr) = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedPhoenixPublicKey)
        }
    }

    /// Returns the inner public key, if present.
    ///
    /// # Errors
    /// If the address is a shielded one.
    pub fn public_key(&self) -> Result<&BlsPublicKey, Error> {
        if let Self::Public(addr) = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedBlsPublicKey)
        }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Shielded(addr) => addr.to_bytes().to_vec(),
            Self::Public(addr) => addr.to_bytes().to_vec(),
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

impl From<BlsPublicKey> for Address {
    fn from(value: BlsPublicKey) -> Self {
        Self::Public(value)
    }
}

impl From<PhoenixPublicKey> for Address {
    fn from(value: PhoenixPublicKey) -> Self {
        Self::Shielded(value)
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address_bytes = bs58::decode(s).into_vec()?;

        let address = match address_bytes.len() {
            PhoenixPublicKey::SIZE => {
                PhoenixPublicKey::from_slice(&address_bytes)?.into()
            }
            BlsPublicKey::SIZE => {
                BlsPublicKey::from_slice(&address_bytes)?.into()
            }
            _ => return Err(Error::Bytes(dusk_bytes::Error::InvalidData)),
        };
        Ok(address)
    }
}

impl From<&Address> for String {
    fn from(address: &Address) -> Self {
        match address {
            Address::Shielded(addr) => {
                bs58::encode(addr.to_bytes()).into_string()
            }
            Address::Public(addr) => {
                bs58::encode(addr.to_bytes()).into_string()
            }
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Address::Shielded(self_pk), Address::Shielded(other_pk)) => {
                self_pk == other_pk
            }
            (Address::Public(self_pk), Address::Public(other_pk)) => {
                self_pk == other_pk
            }
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
            Address::Shielded(self.shielded_addr)
        )
    }

    /// Format the public account into a string.
    pub fn public_account_string(&self) -> String {
        format!(
            "{} - {}",
            public_account_prefix(),
            Address::Public(self.public_addr)
        )
    }

    /// Format the staking account into a string.
    pub fn staking_account_string(&self) -> String {
        format!(
            "{} - {}",
            staking_account_prefix(),
            Address::Public(self.public_addr)
        )
    }

    /// Format the shortened shielded address into a string.
    pub fn shielded_address_preview(&self) -> String {
        format!(
            "{} - {}",
            shielded_address_prefix(),
            Address::Shielded(self.shielded_addr).preview(),
        )
    }

    /// Format the shortened public account into a string.
    pub fn public_account_preview(&self) -> String {
        format!(
            "{} - {}",
            public_account_prefix(),
            Address::Public(self.public_addr).preview()
        )
    }

    /// Format the shortened staking account into a string.
    pub fn staking_account_preview(&self) -> String {
        format!(
            "{} - {}",
            staking_account_prefix(),
            Address::Public(self.public_addr).preview()
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
