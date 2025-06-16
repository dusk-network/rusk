// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, BlockModeError, Cbc, InvalidKeyIvLength};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use rand::rngs::{OsRng, StdRng};
use rand::RngCore;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

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
    let ciphertext = fs::read(&path).map_err(|e| {
        anyhow::anyhow!(
            "{} should be valid consensus keys file {e}",
            path.display()
        )
    })?;

    let aes_key = hash_sha256(pwd);

    let bytes = decrypt(&ciphertext[..], &aes_key)
        .map_err(|e| anyhow::anyhow!("Invalid consensus keys password {e}"))?;

    let keys: BlsKeyPair = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("keys files should contain json {e}"))?;

    let sk = BlsSecretKey::from_slice(&keys.secret_key_bls)
        .map_err(|e| anyhow::anyhow!("sk should be valid {e:?}"))?;

    let pk = BlsPublicKey::from_slice(&keys.public_key_bls)
        .map_err(|e| anyhow::anyhow!("pk should be valid {e:?}"))?;

    Ok((pk, sk))
}

pub fn save_consensus_keys(
    path: &Path,
    filename: &str,
    pk: &BlsPublicKey,
    sk: &BlsSecretKey,
    pwd: &str,
) -> Result<(PathBuf, PathBuf), ConsensusKeysError> {
    let path = path.join(filename);
    let bytes = pk.to_bytes();
    fs::write(path.with_extension("cpk"), bytes)?;

    let mut bls = BlsKeyPair {
        public_key_bls: pk.to_bytes().to_vec(),
        secret_key_bls: sk.to_bytes().to_vec(),
    };

    let mut bytes = serde_json::to_vec(&bls)?;
    let aes_key = hash_sha256(pwd);
    let encrypted_bytes = encrypt(&bytes, &aes_key)?;

    fs::write(path.with_extension("keys"), encrypted_bytes)?;

    bls.secret_key_bls.zeroize();
    bytes.zeroize();

    Ok((path.with_extension("keys"), path.with_extension("cpk")))
}

fn hash_sha256(pwd: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    hasher.finalize().to_vec()
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct BlsKeyPair {
    #[serde_as(as = "Base64")]
    secret_key_bls: Vec<u8>,
    #[serde_as(as = "Base64")]
    public_key_bls: Vec<u8>,
}

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

fn encrypt(
    plaintext: &[u8],
    pwd: &[u8],
) -> Result<Vec<u8>, ConsensusKeysError> {
    let mut iv = vec![0; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut iv);

    let cipher = Aes256Cbc::new_from_slices(pwd, &iv)?;
    let enc = cipher.encrypt_vec(plaintext);

    let ciphertext = iv.into_iter().chain(enc).collect();
    Ok(ciphertext)
}

fn decrypt(data: &[u8], pwd: &[u8]) -> Result<Vec<u8>, BlockModeError> {
    let iv = &data[..16];
    let enc = &data[16..];

    let cipher = Aes256Cbc::new_from_slices(pwd, iv).expect("valid data");
    cipher.decrypt_vec(enc)
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusKeysError {
    #[error("Consensus keys file corrupted")]
    InvalidKeyIvLength(#[from] InvalidKeyIvLength),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_save_load_consensus_keys() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;

        let mut rng = StdRng::seed_from_u64(64);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = BlsPublicKey::from(&sk);
        let pwd = "password";

        save_consensus_keys(dir.path(), "consensus", &pk, &sk, pwd)?;
        let keys_path = dir.path().join("consensus.keys");
        let (loaded_sk, loaded_pk) = load_keys(
            keys_path
                .to_str()
                .ok_or(anyhow!("Failed to convert path to string"))?
                .to_string(),
            pwd.to_string(),
        )?;
        let pk_bytes = fs::read(dir.path().join("consensus.cpk"))?;
        let pk_bytes: [u8; PUBLIC_BLS_SIZE] = pk_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid BlsPublicKey bytes"))?;
        let loaded_cpk = BlsPublicKey::from_bytes(&pk_bytes)
            .map_err(|err| anyhow!("{err:?}"))?;

        assert_eq!(loaded_sk, sk);
        assert_eq!(loaded_pk.inner, pk);
        assert_eq!(loaded_cpk, pk);

        Ok(())
    }

    #[test]
    fn test_can_still_load_keys_saved_by_wallet_impl(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // test-data/wallet-generated-consensus-keys contains consensus keys
        // exported by the former rusk-wallet implementation to save consensus
        // keys.
        // This test checks if what is saved by the former implementation
        // is still loaded correctly.
        let mut rng = StdRng::seed_from_u64(64);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = BlsPublicKey::from(&sk);

        let pwd = "password".to_string();
        let wallet_gen_keys_path = get_wallet_gen_consensus_keys_path();
        let (loaded_sk, loaded_pk) =
            load_keys(wallet_gen_keys_path.to_str().unwrap().to_string(), pwd)?;

        assert_eq!(loaded_sk, sk);
        assert_eq!(loaded_pk.inner, pk);

        Ok(())
    }

    fn get_wallet_gen_consensus_keys_path() -> PathBuf {
        let mut path = PathBuf::from(file!());
        // Remove the filename
        path.pop();
        // Remove the current directory
        let path: PathBuf = path.components().skip(1).collect();
        path.join("test-data")
            .join("wallet-generated-consensus-keys")
            .join("consensus.keys")
    }
}
