// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! File & key management for provisioner nodes.

use std::fs;
use std::path::{Path, PathBuf};

use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use tracing::info;
use zeroize::Zeroize;

use crate::Error;
use crate::crypto::{
    aes_gcm_gen_iv, aes_gcm_gen_salt, decrypt_aes_cbc, decrypt_aes_gcm,
    derive_aes_key, encrypt_aes_gcm, hash_sha256,
};
use crate::{IV_SIZE, SALT_SIZE};

#[serde_as]
#[derive(Serialize, Deserialize)]
struct ProvisionerFileContents {
    #[serde_as(as = "Base64")]
    salt: [u8; SALT_SIZE],
    #[serde_as(as = "Base64")]
    iv: [u8; IV_SIZE],
    /// A `BlsKeyPair` encrypted using AES-GCM
    key_pair: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct BlsKeyPair {
    #[serde_as(as = "Base64")]
    secret_key_bls: Vec<u8>,
    #[serde_as(as = "Base64")]
    public_key_bls: Vec<u8>,
}

impl TryFrom<&BlsKeyPair> for (BlsSecretKey, BlsPublicKey) {
    type Error = Error;

    fn try_from(keys: &BlsKeyPair) -> Result<Self, Self::Error> {
        let mut sk = BlsSecretKey::from_slice(&keys.secret_key_bls)
            .map_err(|_| Error::CorruptedData)?;
        let pk =
            BlsPublicKey::from_slice(&keys.public_key_bls).map_err(|_| {
                sk.zeroize();
                Error::CorruptedData
            })?;
        Ok((sk, pk))
    }
}

/// Loads BLS consensus keys from an encrypted file.
///
/// This function reads consensus keys from a file that was previously saved
/// using [`save_consensus_keys`]. The file is expected to be encrypted with
/// AES-GCM encryption using the provided password.
///
/// The function also handles backward compatibility with older file formats. If
/// an old format is detected, it will automatically migrate the file to the new
/// format while preserving the original as a `.old` backup.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The password is incorrect
/// - The file is corrupted or has invalid key data
/// - File migration operations fail (for old format files)
pub fn load_keys(
    path: &str,
    pwd: &str,
) -> Result<(BlsSecretKey, BlsPublicKey), Error> {
    let path_buf = PathBuf::from(path);
    let (pk, sk) = read_from_file(&path_buf, pwd)?;

    Ok((sk, pk))
}

/// Fetches BLS public and secret keys from an encrypted consensus keys file.
fn read_from_file(
    path: &Path,
    pwd: &str,
) -> Result<(BlsPublicKey, BlsSecretKey), Error> {
    let contents = fs::read(path)?;

    let (mut bytes, file_format_is_old) = if let Ok(contents) =
        serde_json::from_slice::<ProvisionerFileContents>(&contents)
    {
        let mut aes_key = derive_aes_key(pwd, &contents.salt);
        let bytes =
            decrypt_aes_gcm(&contents.key_pair, &aes_key, &contents.iv)?;
        aes_key.zeroize();
        (bytes, false)
    } else {
        let mut aes_key = hash_sha256(pwd);
        let bytes = decrypt_aes_cbc(&contents, &aes_key)?;
        aes_key.zeroize();
        (bytes, true)
    };

    let mut keys: BlsKeyPair = serde_json::from_slice(&bytes)?;
    bytes.zeroize();
    let (mut sk, pk) = TryFrom::<&BlsKeyPair>::try_from(&keys)
        .inspect_err(|_| keys.secret_key_bls.zeroize())?;

    if file_format_is_old {
        migrate_file(path, &pk, &sk, pwd).inspect_err(|_| sk.zeroize())?;
    }

    Ok((pk, sk))
}

/// Saves the consensus keys to disk in encrypted format.
///
/// This function saves both the BLS public key and secret key to separate
/// files:
/// - The public key is saved as a `.cpk` file in plain text
/// - The public and secret keys are saved, along with the IV and salt, in a
///   JSON `.keys` file, with the public and secret keys encrypted using AES-GCM with the
///   provided password.
///
/// Their paths are returned as (.keys file path, .cpk public key path)
///
/// # Errors
///
/// Returns an error if:
/// - File system operations fail
/// - Encryption operations fail
/// - JSON serialization fails.
pub fn save_consensus_keys(
    path: &Path,
    filename: &str,
    pk: &BlsPublicKey,
    sk: &BlsSecretKey,
    pwd: &str,
) -> Result<(PathBuf, PathBuf), Error> {
    let path = path.join(filename);
    let bytes = pk.to_bytes();
    fs::write(path.with_extension("cpk"), bytes)?;

    let iv = aes_gcm_gen_iv();
    let salt = aes_gcm_gen_salt();
    let mut bls = BlsKeyPair {
        public_key_bls: pk.to_bytes().to_vec(),
        secret_key_bls: sk.to_bytes().to_vec(),
    };
    let key_pair_plain = serde_json::to_vec(&bls);
    bls.secret_key_bls.zeroize();
    let mut key_pair_plain = key_pair_plain?;

    let mut aes_key = derive_aes_key(pwd, &salt);
    let key_pair_enc = encrypt_aes_gcm(&key_pair_plain, &aes_key, &iv);
    aes_key.zeroize();
    key_pair_plain.zeroize();
    let contents = serde_json::to_vec(&ProvisionerFileContents {
        salt,
        iv,
        key_pair: key_pair_enc?,
    })?;

    fs::write(path.with_extension("keys"), contents)?;

    Ok((path.with_extension("keys"), path.with_extension("cpk")))
}

fn migrate_file(
    path: &Path,
    pk: &BlsPublicKey,
    sk: &BlsSecretKey,
    pwd: &str,
) -> Result<(), Error> {
    info!(
        "Your consensus keys are in the old format. Migrating to the new format and saving the old file as {}.old",
        path.display()
    );
    save_old_file(path)?;
    let keys_filename = path
        .file_name()
        .ok_or(Error::InvalidKeysFilePath)?
        .to_str()
        .ok_or(Error::InvalidKeysFilePath)?;
    let keys_file_dir = path.parent().ok_or(Error::InvalidKeysFilePath)?;
    save_consensus_keys(keys_file_dir, keys_filename, pk, sk, pwd)?;
    info!("Migration complete");

    Ok(())
}

fn save_old_file(path: &Path) -> Result<(), Error> {
    let old_path = path.with_extension("keys.old");
    fs::copy(path, old_path)?;
    Ok(())
}
