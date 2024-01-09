// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, BlockModeError, Cbc};
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable;

use rand::rngs::StdRng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use tracing::warn;

pub const PUBLIC_BLS_SIZE: usize = dusk_bls12_381_sign::PublicKey::SIZE;

/// Extends dusk_bls12_381_sign::PublicKey by implementing a few traits
///
/// See also PublicKey::bytes(&self)
#[derive(Default, Eq, PartialEq, Clone)]
pub struct PublicKey {
    inner: dusk_bls12_381_sign::PublicKey,
    as_bytes: PublicKeyBytes,
}

impl TryFrom<[u8; 96]> for PublicKey {
    type Error = dusk_bls12_381_sign::Error;
    fn try_from(bytes: [u8; 96]) -> Result<Self, Self::Error> {
        let inner = dusk_bls12_381_sign::PublicKey::from_slice(&bytes)?;
        let as_bytes = PublicKeyBytes(bytes);
        Ok(Self { as_bytes, inner })
    }
}

impl PublicKey {
    pub fn new(inner: dusk_bls12_381_sign::PublicKey) -> Self {
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
        let sk = SecretKey::random(rng);

        Self::new(dusk_bls12_381_sign::PublicKey::from(&sk))
    }

    /// `bytes` returns a reference to the pk.to_bytes() initialized on
    /// PublicKey::new call. NB Frequent use of `to_bytes()` creates a
    /// noticeable perf overhead.
    pub fn bytes(&self) -> &PublicKeyBytes {
        &self.as_bytes
    }

    pub fn inner(&self) -> &dusk_bls12_381_sign::PublicKey {
        &self.inner
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
        self.as_bytes.inner().partial_cmp(other.as_bytes.inner())
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
/// a wrapper of 96-sized array
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct PublicKeyBytes(pub [u8; PUBLIC_BLS_SIZE]);

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

/// Loads consensus keys from an encrypted file.
///
/// Panics on any error.
pub fn load_keys(
    path: String,
    pwd: String,
) -> (dusk_bls12_381_sign::SecretKey, PublicKey) {
    let path_buf = PathBuf::from(path);
    let (pk, sk) = read_from_file(path_buf, &pwd);

    (sk, PublicKey::new(pk))
}

/// Fetches BLS public and secret keys from an encrypted consensus keys file.
///
/// Panics on any error.
fn read_from_file(
    path: PathBuf,
    pwd: &str,
) -> (
    dusk_bls12_381_sign::PublicKey,
    dusk_bls12_381_sign::SecretKey,
) {
    use serde::Deserialize;

    /// Bls key pair helper structure
    #[derive(Deserialize)]
    struct BlsKeyPair {
        secret_key_bls: String,
        public_key_bls: String,
    }

    // attempt to load and decode wallet
    println!("{path:?}");
    let ciphertext =
        fs::read(path).expect("path should be valid consensus keys file");

    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    let hashed_pwd = hasher.finalize().to_vec();

    let bytes = decrypt(&ciphertext[..], &hashed_pwd).unwrap_or_else(|_| {
        let hashed_pwd = blake3::hash(pwd.as_bytes());
        let bytes = decrypt(&ciphertext[..], hashed_pwd.as_bytes())
            .expect("Invalid consensus keys password");
        warn!("Your consensus keys are in the old format");
        warn!("Consider to export them using a new version of the wallet");
        bytes
    });

    let keys: BlsKeyPair =
        serde_json::from_slice(&bytes).expect("keys files should contain json");

    let sk = dusk_bls12_381_sign::SecretKey::from_slice(
        &base64::decode(keys.secret_key_bls).expect("sk should be base64")[..],
    )
    .expect("sk should be valid");

    let pk = dusk_bls12_381_sign::PublicKey::from_slice(
        &base64::decode(keys.public_key_bls).expect("pk should be base64")[..],
    )
    .expect("pk should be valid");

    (pk, sk)
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
/// It reads RUSK_WALLET_PWD var to unlock wallet files.
pub fn load_provisioners_keys(
    n: usize,
) -> Vec<(dusk_bls12_381_sign::SecretKey, PublicKey)> {
    let mut keys = vec![];

    let dir = std::env::var("DUSK_WALLET_DIR").unwrap();
    let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS").unwrap();

    for i in 0..n {
        let mut path = dir.clone();
        path.push_str(&format!("node_{i}.keys"));
        let path_buf = PathBuf::from(path);

        let (pk, sk) = read_from_file(path_buf, &pwd);

        keys.push((sk, PublicKey::new(pk)));
    }

    keys
}
