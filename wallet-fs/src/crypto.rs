// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::Error;
use crate::{IV_SIZE, PBKDF2_ROUNDS, SALT_SIZE};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Encrypts the plaintext using AES-GCM.
pub(crate) fn encrypt_aes_gcm(
    plaintext: &[u8],
    key: &[u8],
    iv: &[u8],
) -> Result<Vec<u8>, Error> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let iv = aes_gcm::Nonce::from_slice(iv);
    let ciphertext = cipher.encrypt(iv, plaintext)?;
    Ok(ciphertext)
}

/// Decrypts the ciphertext with AES-CBC.
pub(crate) fn decrypt_aes_cbc(
    ciphertext: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, Error> {
    const OLD_IV_SIZE: usize = 16;
    if ciphertext.len() < OLD_IV_SIZE {
        return Err(Error::CorruptedData);
    }
    let iv = &ciphertext[..OLD_IV_SIZE];
    let enc = &ciphertext[OLD_IV_SIZE..];

    let cipher = Aes256Cbc::new_from_slices(key, iv)?;
    let plaintext = cipher.decrypt_vec(enc)?;

    Ok(plaintext)
}

/// Decrypts the ciphertext with AES-GCM.
pub(crate) fn decrypt_aes_gcm(
    ciphertext: &[u8],
    key: &[u8],
    iv: &[u8],
) -> Result<Vec<u8>, Error> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let iv = aes_gcm::Nonce::from_slice(iv);
    let plaintext = cipher.decrypt(iv, ciphertext)?;

    Ok(plaintext)
}

pub(crate) fn derive_aes_key(pwd: &str, salt: &[u8]) -> Vec<u8> {
    pbkdf2::pbkdf2_hmac_array::<Sha256, SALT_SIZE>(
        pwd.as_bytes(),
        salt,
        PBKDF2_ROUNDS,
    )
    .to_vec()
}

pub(crate) fn aes_gcm_gen_iv() -> [u8; IV_SIZE] {
    let iv = Aes256Gcm::generate_nonce(OsRng);
    iv.into()
}

pub(crate) fn aes_gcm_gen_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0; SALT_SIZE];
    let mut rng = OsRng;
    rng.fill_bytes(&mut salt);
    salt
}

pub(crate) fn hash_sha256(pwd: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(pwd.as_bytes());
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use rand::RngCore;
    use rand::rngs::OsRng;

    use super::*;

    #[test]
    fn encrypt_and_decrypt() {
        let seed =
            b"0001020304050607000102030405060700010203040506070001020304050607";
        let key = hash_sha256("greatpassword");
        let iv1 = aes_gcm_gen_iv();
        let iv2 = aes_gcm_gen_iv();

        let enc_seed_cbc =
            encrypt_aes_cbc(seed, &key).expect("seed to encrypt ok");
        let enc_seed_t_cbc =
            encrypt_aes_cbc(seed, &key).expect("seed to encrypt ok");

        let enc_seed_gcm =
            encrypt_aes_gcm(seed, &key, &iv1).expect("seed to encrypt ok");
        let enc_seed_t_gcm =
            encrypt_aes_gcm(seed, &key, &iv2).expect("seed to encrypt ok");

        // check that random IV is correctly applied
        assert_ne!(enc_seed_cbc, enc_seed_t_cbc);
        assert_ne!(enc_seed_gcm, enc_seed_t_gcm);

        let dec_seed_cbc =
            decrypt_aes_cbc(&enc_seed_cbc, &key).expect("seed to decrypt ok");
        let dec_seed_gcm = decrypt_aes_gcm(&enc_seed_gcm, &key, &iv1)
            .expect("seed to decrypt ok");

        // check that decryption matches original seed
        assert_eq!(dec_seed_cbc, seed);
        assert_eq!(dec_seed_gcm, seed);
    }

    // Old `encrypt` implementation.
    fn encrypt_aes_cbc(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, Error> {
        let mut iv = vec![0; 16];
        let mut rng = OsRng;
        rng.fill_bytes(&mut iv);

        let cipher = Aes256Cbc::new_from_slices(key, &iv)?;
        let enc = cipher.encrypt_vec(plaintext);

        let ciphertext = iv.into_iter().chain(enc).collect();
        Ok(ciphertext)
    }
}
