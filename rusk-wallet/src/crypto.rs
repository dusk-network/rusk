// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::rngs::OsRng;

use crate::Error;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Encrypts data using a password.
pub(crate) fn encrypt(plaintext: &[u8], pwd: &[u8]) -> Result<Vec<u8>, Error> {
    let key = Key::<Aes256Gcm>::from_slice(pwd);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(OsRng);
    let enc = cipher.encrypt(&nonce, plaintext)?;
    let ciphertext = nonce.into_iter().chain(enc).collect();
    Ok(ciphertext)
}

/// Decrypts data encrypted with the older AES-CBC `encrypt`.
pub(crate) fn decrypt_aes_cbc(
    ciphertext: &[u8],
    pwd: &[u8],
) -> Result<Vec<u8>, Error> {
    let iv = &ciphertext[..16];
    let enc = &ciphertext[16..];

    let cipher = Aes256Cbc::new_from_slices(pwd, iv)?;
    let plaintext = cipher.decrypt_vec(enc)?;

    Ok(plaintext)
}

/// Decrypts data encrypted with `encrypt`.
pub(crate) fn decrypt_aes_gcm(
    ciphertext: &[u8],
    pwd: &[u8],
) -> Result<Vec<u8>, Error> {
    let nonce = &ciphertext[..12];
    let enc = &ciphertext[12..];

    let key = Key::<Aes256Gcm>::from_slice(pwd);
    let cipher = Aes256Gcm::new(key);
    let nonce = aes_gcm::Nonce::from_slice(nonce);
    let plaintext = cipher.decrypt(nonce, enc)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn encrypt_and_decrypt() {
        let seed =
            b"0001020304050607000102030405060700010203040506070001020304050607";
        let pwd = blake3::hash("greatpassword".as_bytes());
        let pwd = pwd.as_bytes();

        let encrypt_aes_gcm = encrypt;

        let enc_seed_cbc =
            encrypt_aes_cbc(seed, pwd).expect("seed to encrypt ok");
        let enc_seed_t_cbc =
            encrypt_aes_cbc(seed, pwd).expect("seed to encrypt ok");
        let enc_seed_gcm =
            encrypt_aes_gcm(seed, pwd).expect("seed to encrypt ok");
        let enc_seed_t_gcm =
            encrypt_aes_gcm(seed, pwd).expect("seed to encrypt ok");

        // check that random IV is correctly applied
        assert_ne!(enc_seed_cbc, enc_seed_t_cbc);
        assert_ne!(enc_seed_gcm, enc_seed_t_gcm);

        let dec_seed_cbc =
            decrypt_aes_cbc(&enc_seed_cbc, pwd).expect("seed to decrypt ok");
        let dec_seed_gcm =
            decrypt_aes_gcm(&enc_seed_gcm, pwd).expect("seed to decrypt ok");

        // check that decryption matches original seed
        assert_eq!(dec_seed_cbc, seed);
        assert_eq!(dec_seed_gcm, seed);
    }

    // Old `encrypt` implementation.
    pub fn encrypt_aes_cbc(
        plaintext: &[u8],
        pwd: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let mut iv = vec![0; 16];
        let mut rng = OsRng;
        rng.fill_bytes(&mut iv);

        let cipher = Aes256Cbc::new_from_slices(pwd, &iv)?;
        let enc = cipher.encrypt_vec(plaintext);

        let ciphertext = iv.into_iter().chain(enc).collect();
        Ok(ciphertext)
    }
}
