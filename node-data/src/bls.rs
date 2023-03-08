// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::Serializable;
use hex::ToHex;
use rand::rngs::StdRng;
use rand_core::SeedableRng;
use std::cmp::Ordering;

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use dusk_bytes::DeserializableSlice;
use std::fs;
use std::path::PathBuf;

pub const PUBLIC_BLS_SIZE: usize = dusk_bls12_381_sign::PublicKey::SIZE;

/// Extends dusk_bls12_381_sign::PublicKey by implementing a few traits
///
/// See also PublicKey::bytes(&self)
#[derive(Eq, PartialEq, Clone)]
pub struct PublicKey {
    inner: dusk_bls12_381_sign::PublicKey,
    data: [u8; PUBLIC_BLS_SIZE],
}

impl PublicKey {
    pub fn new(inner: dusk_bls12_381_sign::PublicKey) -> Self {
        let data = inner.to_bytes();
        Self { inner, data }
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
    pub fn bytes(&self) -> &[u8; PUBLIC_BLS_SIZE] {
        &self.data
    }

    pub fn inner(&self) -> &dusk_bls12_381_sign::PublicKey {
        &self.inner
    }

    pub fn encode_short_hex(&self) -> String {
        let mut hex = self.bytes().encode_hex::<String>();
        hex.truncate(16);
        hex
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        Self {
            inner: dusk_bls12_381_sign::PublicKey::default(),
            data: [0; PUBLIC_BLS_SIZE],
        }
    }
}

impl PartialOrd<PublicKey> for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut hex = self.data.encode_hex::<String>();
        hex.truncate(16);

        let debug_trait_builder =
            &mut ::core::fmt::Formatter::debug_tuple(f, "PublicKey");
        let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &hex);
        ::core::fmt::DebugTuple::finish(debug_trait_builder)
    }
}

/// Loads consensus keys from an encrypted file.
///
/// Panics on any error.
pub fn load_keys(
    path: String,
    pwd: String,
) -> (dusk_bls12_381_sign::SecretKey, PublicKey) {
    let pwd = blake3::hash(pwd.as_bytes());

    let path_buf = PathBuf::from(path);
    let (pk, sk) = read_from_file(path_buf, pwd);

    (sk, PublicKey::new(pk))
}

/// Fetches BLS public and secret keys from an encrypted consensus keys file.
///
/// Panics on any error.
pub fn read_from_file(
    path: PathBuf,
    pwd: blake3::Hash,
) -> (
    dusk_bls12_381_sign::PublicKey,
    dusk_bls12_381_sign::SecretKey,
) {
    use serde::Deserialize;
    type Aes256Cbc = Cbc<Aes256, Pkcs7>;

    /// Bls key pair helper structure
    #[derive(Deserialize)]
    struct BlsKeyPair {
        secret_key_bls: String,
        public_key_bls: String,
    }

    // attempt to load and decode wallet
    let ciphertext =
        fs::read(&path).expect("path should be valid consensus keys file");

    // Decrypt
    let iv = &ciphertext[..16];
    let enc = &ciphertext[16..];

    let cipher =
        Aes256Cbc::new_from_slices(pwd.as_bytes(), iv).expect("valid data");
    let bytes = cipher.decrypt_vec(enc).expect("pwd should be valid");

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
