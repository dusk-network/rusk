// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::fmt::Debug;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::Serialize;

pub const PUBLIC_BLS_SIZE: usize = BlsPublicKey::SIZE;

/// Extends BlsPublicKey by implementing a few traits
///
/// See also PublicKey::bytes(&self)
#[derive(Default, Eq, PartialEq, Clone)]
pub struct PublicKey {
    inner: BlsPublicKey,
    as_bytes: PublicKeyBytes,
}

impl TryFrom<[u8; 96]> for PublicKey {
    type Error = dusk_bytes::Error;
    fn try_from(bytes: [u8; 96]) -> Result<Self, Self::Error> {
        let inner = BlsPublicKey::from_slice(&bytes)?;
        let as_bytes = PublicKeyBytes(bytes);
        Ok(Self { as_bytes, inner })
    }
}

impl PublicKey {
    pub fn new(inner: BlsPublicKey) -> Self {
        let b = inner.to_bytes();
        Self {
            inner,
            as_bytes: PublicKeyBytes(b),
        }
    }

    /// from_sk_seed_u64 generates a sk from the specified seed and returns the
    /// associated public key
    pub fn from_sk_seed_u64(state: u64) -> Self {
        let rng = &mut StdRng::seed_from_u64(state);
        let sk = BlsSecretKey::random(rng);

        Self::new(BlsPublicKey::from(&sk))
    }

    /// `bytes` returns a reference to the pk.to_bytes() initialized on
    /// PublicKey::new call. NB: Frequent use of `to_bytes()` creates a
    /// noticeable performance overhead.
    pub fn bytes(&self) -> &PublicKeyBytes {
        &self.as_bytes
    }

    pub fn inner(&self) -> &BlsPublicKey {
        &self.inner
    }

    pub fn into_inner(self) -> BlsPublicKey {
        self.inner
    }

    /// Truncated base58 representation of inner data
    pub fn to_bs58(&self) -> String {
        self.bytes().to_bs58()
    }

    /// Full base58 representation of inner data
    pub fn to_base58(&self) -> String {
        self.bytes().to_base58()
    }
}

impl PartialOrd<PublicKey> for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes.inner().cmp(other.as_bytes.inner())
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let bs = self.to_base58();
        f.debug_struct("PublicKey").field("bs58", &bs).finish()
    }
}
/// A wrapper of 96-sized array
#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct PublicKeyBytes(
    #[serde(serialize_with = "crate::serialize_b58")] pub [u8; PUBLIC_BLS_SIZE],
);

impl Default for PublicKeyBytes {
    fn default() -> Self {
        PublicKeyBytes([0; 96])
    }
}

impl PublicKeyBytes {
    pub fn inner(&self) -> &[u8; 96] {
        &self.0
    }

    /// Full base58 representation of inner data
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Truncated base58 representation of inner data
    pub fn to_bs58(&self) -> String {
        let mut bs = self.to_base58();
        bs.truncate(16);
        bs
    }
}

impl Debug for PublicKeyBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_bs58())
    }
}

/// Loads consensus keys from an encrypted file.
pub fn load_keys(
    path: String,
    pwd: String,
) -> anyhow::Result<(BlsSecretKey, PublicKey)> {
    match wallet_fs::provisioner::load_keys(&path, &pwd) {
        Ok((sk, pk)) => Ok((sk, PublicKey::new(pk))),
        Err(e) => match e {
            wallet_fs::Error::CorruptedData => {
                Err(anyhow::anyhow!("The provisioner keys file is corrupted."))
            }
            wallet_fs::Error::EncryptDecryptFailure => Err(anyhow::anyhow!(
                "Failed to decrypt: invalid consensus keys password or the file is corrupted"
            )),
            err => Err(anyhow::anyhow!("{err}")),
        },
    }
}
