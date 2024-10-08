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

/// Address for which to perform transactions with
/// it may be owned by the user or not, if the address is a receiver
/// then the index field will be none
#[derive(Clone, Eq)]
#[allow(missing_docs)]
pub enum Address {
    /// A Phoenix address used for Phoenix transaction
    Phoenix { pk: PhoenixPublicKey },
    /// A BLS address used for Moonlight transactions and staking operations
    Bls { pk: BlsPublicKey },
}

/// A public address within Dusk
impl Address {
    /// Returns the phoenix-key of the Address if there is any.
    ///
    /// # Errors
    /// If the address carries a bls-key.
    pub fn try_phoenix_pk(&self) -> Result<&PhoenixPublicKey, Error> {
        if let Self::Phoenix { pk } = self {
            Ok(pk)
        } else {
            Err(Error::ExpectedPhoenixPublicKey)
        }
    }

    /// Returns the bls-key of the Address if there is any.
    ///
    /// # Errors
    /// If the address carries a phoenix-key.
    pub fn try_bls_pk(&self) -> Result<&BlsPublicKey, Error> {
        if let Self::Bls { pk } = self {
            Ok(pk)
        } else {
            Err(Error::ExpectedBlsPublicKey)
        }
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Phoenix { pk } => pk.to_bytes().to_vec(),
            Self::Bls { pk } => pk.to_bytes().to_vec(),
        }
    }

    // Returns a string of 23 character specifying the address kind (Phoenix or
    // Moonlight/Stake for Bls)
    fn addr_kind_str(&self) -> String {
        match self {
            Address::Phoenix { pk: _ } => "Phoenix Address".to_string(),
            Address::Bls { pk: _ } => "Moonlight/Stake Address".to_string(),
        }
    }

    /// A trimmed version of the address to display as preview
    pub fn preview(&self) -> String {
        let addr_key_str = String::from(self);
        format!(
            "{:<23} - {}...{}",
            self.addr_kind_str(),
            &addr_key_str[..7],
            &addr_key_str[addr_key_str.len() - 7..]
        )
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address_bytes = bs58::decode(s).into_vec()?;
        let mut address_reader = &address_bytes[..];

        match address_bytes.len() {
            PhoenixPublicKey::SIZE => Ok(Self::Phoenix {
                pk: PhoenixPublicKey::from_reader(&mut address_reader)
                    .map_err(Error::Bytes)?,
            }),
            BlsPublicKey::SIZE => Ok(Self::Bls {
                pk: BlsPublicKey::from_reader(&mut address_reader)
                    .map_err(Error::Bytes)?,
            }),
            _ => Err(Error::Bytes(dusk_bytes::Error::InvalidData)),
        }
    }
}

impl From<&Address> for String {
    fn from(address: &Address) -> Self {
        match address {
            Address::Phoenix { pk } => {
                bs58::encode(pk.to_bytes()).into_string()
            }
            Address::Bls { pk } => bs58::encode(pk.to_bytes()).into_string(),
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Address::Phoenix { pk: self_pk },
                Address::Phoenix { pk: other_pk },
            ) => self_pk == other_pk,
            (Address::Bls { pk: self_pk }, Address::Bls { pk: other_pk }) => {
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
        write!(f, "{:<23} - {}", self.addr_kind_str(), String::from(self))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<23} - {}", self.addr_kind_str(), String::from(self))
    }
}
