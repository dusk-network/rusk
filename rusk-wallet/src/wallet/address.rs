// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::hash::Hasher;
use std::{fmt, str::FromStr};

use super::*;
use crate::Error;

use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};

/// Address for which to perform transactions with
/// it may be owned by the user or not, if the address is a reciever
/// then the index field will be none
#[derive(Clone, Eq)]
#[allow(missing_docs)]
pub enum Address {
    /// A Phoenix address
    Phoenix {
        index: Option<u8>,
        addr: PhoenixPublicKey,
    },
    /// A BLS address for moonlight account
    Bls {
        index: Option<u8>,
        addr: AccountPublicKey,
    },
}

/// A public address within the Dusk Network
impl Address {
    /// Returns true if the current user owns this address
    pub fn is_owned(&self) -> bool {
        self.index().is_ok()
    }

    pub(crate) fn pk(&self) -> Result<&PhoenixPublicKey, Error> {
        if let Self::Phoenix { addr, .. } = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedPhoenixPublicKey)
        }
    }

    pub(crate) fn apk(&self) -> Result<&AccountPublicKey, Error> {
        if let Self::Bls { addr, .. } = self {
            Ok(addr)
        } else {
            Err(Error::ExpectedBlsPublicKey)
        }
    }

    /// find index of the address
    pub fn index(&self) -> Result<u8, Error> {
        match self {
            Self::Phoenix { index, .. } => index,
            Self::Bls { index, .. } => index,
        }
        .ok_or(Error::AddressNotOwned)
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Phoenix { addr, .. } => addr.to_bytes().to_vec(),
            Self::Bls { addr, .. } => addr.to_bytes().to_vec(),
        }
    }

    /// A trimmed version of the address to display as preview
    pub fn preview(&self) -> String {
        let bytes = self.to_bytes();
        let addr = bs58::encode(bytes).into_string();
        format!("{}...{}", &addr[..7], &addr[addr.len() - 7..])
    }

    /// try to create phoenix address from string
    pub fn try_from_str_phoenix(s: &str) -> Result<Self, Error> {
        let bytes = bs58::decode(s).into_vec()?;

        let pk = PhoenixPublicKey::from_reader(&mut &bytes[..])
            .map_err(|_| Error::BadAddress)?;

        let addr = Self::Phoenix {
            index: None,
            addr: pk,
        };

        Ok(addr)
    }

    /// try to create moonlight address from string
    pub fn try_from_str_bls(s: &str) -> Result<Self, Error> {
        let bytes = bs58::decode(s).into_vec()?;

        let apk = AccountPublicKey::from_reader(&mut &bytes[..])
            .map_err(|_| Error::BadAddress)?;

        let addr = Self::Bls {
            index: None,
            addr: apk,
        };

        Ok(addr)
    }

    /// try to create phoenix public key from bytes
    pub fn try_from_bytes_phoenix(
        bytes: &[u8; PhoenixPublicKey::SIZE],
    ) -> Result<Self, Error> {
        let addr = Self::Phoenix {
            index: None,
            addr: PhoenixPublicKey::from_bytes(bytes)?,
        };

        Ok(addr)
    }

    /// Create an address instance from `BlsPublicKey` bytes.
    pub fn try_from_bytes_bls(
        bytes: &[u8; AccountPublicKey::SIZE],
    ) -> Result<Self, Error> {
        let addr = Self::Bls {
            index: None,
            addr: AccountPublicKey::from_bytes(bytes)?,
        };

        Ok(addr)
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let try_phoenix = Self::try_from_str_phoenix(s);
        let try_bls = Self::try_from_str_bls(s);

        if let Ok(addr) = try_phoenix {
            Ok(addr)
        } else {
            try_bls
        }
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self.index(), other.index()) {
            (Ok(x), Ok(y)) => x == y && self.preview() == other.preview(),
            (_, _) => self.preview() == other.preview(),
        }
    }
}

impl std::hash::Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // we dont care about addresses we dont own
        let _ = self.index().map(|f| f.hash(state));
        self.preview().hash(state);
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.to_bytes()).into_string())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.to_bytes()).into_string())
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
