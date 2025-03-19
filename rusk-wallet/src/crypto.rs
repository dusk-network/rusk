// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};

use crate::Error;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Encrypts data using a password.
pub(crate) fn encrypt(
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

/// Decrypts data encrypted with the older AES-CBC `encrypt`.
pub(crate) fn decrypt_aes_cbc(
    ciphertext: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, Error> {
    const OLD_IV_SIZE: usize = 16;
    let iv = &ciphertext[..OLD_IV_SIZE];
    let enc = &ciphertext[OLD_IV_SIZE..];

    let cipher = Aes256Cbc::new_from_slices(key, iv)?;
    let plaintext = cipher.decrypt_vec(enc)?;

    Ok(plaintext)
}

/// Decrypts data encrypted with `encrypt`.
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

#[cfg(test)]
mod tests {
    use aes_gcm::AeadCore;
    use rand::rngs::OsRng;
    use rand::RngCore;

    use crate::IV_SIZE;

    use super::*;

    #[test]
    fn encrypt_and_decrypt() {
        let seed =
            b"0001020304050607000102030405060700010203040506070001020304050607";
        let key = blake3::hash("greatpassword".as_bytes());
        let key = key.as_bytes();
        let iv1 = gen_iv();
        let iv2 = gen_iv();

        let encrypt_aes_gcm = encrypt;

        let enc_seed_cbc =
            encrypt_aes_cbc(seed, key).expect("seed to encrypt ok");
        let enc_seed_t_cbc =
            encrypt_aes_cbc(seed, key).expect("seed to encrypt ok");

        let enc_seed_gcm =
            encrypt_aes_gcm(seed, key, &iv1).expect("seed to encrypt ok");
        let enc_seed_t_gcm =
            encrypt_aes_gcm(seed, key, &iv2).expect("seed to encrypt ok");

        // check that random IV is correctly applied
        assert_ne!(enc_seed_cbc, enc_seed_t_cbc);
        assert_ne!(enc_seed_gcm, enc_seed_t_gcm);

        let dec_seed_cbc =
            decrypt_aes_cbc(&enc_seed_cbc, key).expect("seed to decrypt ok");
        let dec_seed_gcm = decrypt_aes_gcm(&enc_seed_gcm, key, &iv1)
            .expect("seed to decrypt ok");

        // check that decryption matches original seed
        assert_eq!(dec_seed_cbc, seed);
        assert_eq!(dec_seed_gcm, seed);
    }

    fn gen_iv() -> [u8; IV_SIZE] {
        let iv = Aes256Gcm::generate_nonce(OsRng);
        iv.into()
    }

    // Old `encrypt` implementation.
    pub fn encrypt_aes_cbc(
        plaintext: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let mut iv = vec![0; 16];
        let mut rng = OsRng;
        rng.fill_bytes(&mut iv);

        let cipher = Aes256Cbc::new_from_slices(key, &iv)?;
        let enc = cipher.encrypt_vec(plaintext);

        let ciphertext = iv.into_iter().chain(enc).collect();
        Ok(ciphertext)
    }
}
