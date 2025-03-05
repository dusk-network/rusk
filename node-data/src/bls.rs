// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use aes::Aes256;
use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, BlockModeError, Cbc};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::warn;
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
    let path_buf = PathBuf::from(path);
    let (pk, sk) = read_from_file(path_buf, &pwd)?;

    Ok((sk, PublicKey::new(pk)))
}

/// Fetches BLS public and secret keys from an encrypted consensus keys file.
fn read_from_file(
    path: PathBuf,
    pwd: &str,
) -> anyhow::Result<(BlsPublicKey, BlsSecretKey)> {
    use serde::Deserialize;

    /// Bls key pair helper structure
    #[derive(Deserialize)]
    struct BlsKeyPair {
        secret_key_bls: String,
        public_key_bls: String,
    }

    // attempt to load and decode wallet
    let ciphertext = fs::read(&path).map_err(|e| {
        anyhow::anyhow!(
            "{} should be valid consensus keys file {e}",
            path.display()
        )
    })?;

    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    let hashed_pwd = hasher.finalize().to_vec();

    let bytes = match decrypt(&ciphertext[..], &hashed_pwd) {
        Ok(bytes) => bytes,
        Err(_) => {
            let bytes = decrypt(&ciphertext[..], &hashed_pwd).map_err(|e| {
                anyhow::anyhow!("Invalid consensus keys password {e}")
            })?;
            warn!("Your consensus keys are in the old format");
            warn!("Consider to export them using a new version of the wallet");
            bytes
        }
    };

    let keys: BlsKeyPair = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("keys files should contain json {e}"))?;

    let sk_bytes = BASE64_ENGINE
        .decode(keys.secret_key_bls)
        .map_err(|e| anyhow::anyhow!("sk should be base64 {e}"))?;

    let sk = BlsSecretKey::from_slice(&sk_bytes)
        .map_err(|e| anyhow::anyhow!("sk should be valid {e:?}"))?;

    let pk = BlsPublicKey::from_slice(
        &BASE64_ENGINE
            .decode(keys.public_key_bls)
            .map_err(|e| anyhow::anyhow!("pk should be base64 {e}"))?[..],
    )
    .map_err(|e| anyhow::anyhow!("pk should be valid {e:?}"))?;

    Ok((pk, sk))
}

fn decrypt(data: &[u8], pwd: &[u8]) -> Result<Vec<u8>, BlockModeError> {
    type Aes256Cbc = Cbc<Aes256, Pkcs7>;
    let iv = &data[..16];
    let enc = &data[16..];

    let cipher = Aes256Cbc::new_from_slices(pwd, iv).expect("valid data");
    cipher.decrypt_vec(enc)
}

/// Loads wallet files from $DUSK_WALLET_DIR and returns a vector of all loaded
/// consensus keys.
///
/// It reads $DUSK_CONSENSUS_KEYS_PASS var to unlock wallet files.
pub fn load_provisioners_keys(n: usize) -> Vec<(BlsSecretKey, PublicKey)> {
    let mut keys = vec![];

    let dir = std::env::var("DUSK_WALLET_DIR").unwrap();
    let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS").unwrap();

    for i in 0..n {
        let mut path = dir.clone();
        path.push_str(&format!("node_{i}.keys"));
        let path_buf = PathBuf::from(path);

        let (pk, sk) = read_from_file(path_buf, &pwd).unwrap();

        keys.push((sk, PublicKey::new(pk)));
    }

    keys
}
