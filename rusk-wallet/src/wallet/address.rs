// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use dusk_pki::PublicSpendKey;
use std::fmt;
use std::hash::Hasher;
use std::str::FromStr;

#[derive(Clone, Eq)]
/// A public address within the Dusk Network
pub struct Address {
    pub(crate) index: Option<u8>,
    pub(crate) psk: PublicSpendKey,
}

impl Address {
    pub(crate) fn new(index: u8, psk: PublicSpendKey) -> Self {
        Self {
            index: Some(index),
            psk,
        }
    }

    /// Returns true if the current user owns this address
    pub fn is_owned(&self) -> bool {
        self.index.is_some()
    }

    pub(crate) fn psk(&self) -> &PublicSpendKey {
        &self.psk
    }

    pub(crate) fn index(&self) -> Result<u8, Error> {
        self.index.ok_or(Error::AddressNotOwned)
    }

    /// A trimmed version of the address to display as preview
    pub fn preview(&self) -> String {
        let addr = bs58::encode(self.psk.to_bytes()).into_string();
        format!("{}...{}", &addr[..7], &addr[addr.len() - 7..])
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;

        let psk = PublicSpendKey::from_reader(&mut &bytes[..])
            .map_err(|_| Error::BadAddress)?;

        let addr = Address { index: None, psk };

        Ok(addr)
    }
}

impl TryFrom<String> for Address {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Address::from_str(s.as_str())
    }
}

impl TryFrom<&[u8; PublicSpendKey::SIZE]> for Address {
    type Error = Error;

    fn try_from(
        bytes: &[u8; PublicSpendKey::SIZE],
    ) -> Result<Self, Self::Error> {
        let addr = Address {
            index: None,
            psk: dusk_pki::PublicSpendKey::from_bytes(bytes)?,
        };
        Ok(addr)
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.psk == other.psk
    }
}

impl std::hash::Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.psk.to_bytes().hash(state);
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.psk.to_bytes()).into_string())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.psk.to_bytes()).into_string())
    }
}

/// Addresses holds address-related metadata that needs to be
/// persisted in the wallet file.
pub(crate) struct Addresses {
    pub(crate) count: u8,
}

impl Default for Addresses {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl Serializable<1> for Addresses {
    type Error = BytesError;

    fn from_bytes(buf: &[u8; Addresses::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self { count: buf[0] })
    }

    fn to_bytes(&self) -> [u8; Addresses::SIZE] {
        [self.count]
    }
}

#[test]
fn addresses_serde() -> Result<(), Box<dyn std::error::Error>> {
    let addrs = Addresses { count: 6 };
    let read = Addresses::from_bytes(&addrs.to_bytes())
        .map_err(|_| Error::WalletFileCorrupted)?;
    assert!(read.count == addrs.count);
    Ok(())
}
