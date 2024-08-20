// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::Error;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Encrypts data using a password.
pub(crate) fn encrypt(plaintext: &[u8], pwd: &[u8]) -> Result<Vec<u8>, Error> {
    let mut iv = vec![0; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut iv);

    let cipher = Aes256Cbc::new_from_slices(pwd, &iv)?;
    let enc = cipher.encrypt_vec(plaintext);

    let ciphertext = iv.into_iter().chain(enc).collect();
    Ok(ciphertext)
}

/// Decrypts data encrypted with `encrypt`.
pub(crate) fn decrypt(ciphertext: &[u8], pwd: &[u8]) -> Result<Vec<u8>, Error> {
    let iv = &ciphertext[..16];
    let enc = &ciphertext[16..];

    let cipher = Aes256Cbc::new_from_slices(pwd, iv)?;
    let plaintext = cipher.decrypt_vec(enc)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_and_decrypt() {
        let seed =
            b"0001020304050607000102030405060700010203040506070001020304050607";
        let pwd = blake3::hash("greatpassword".as_bytes());
        let pwd = pwd.as_bytes();

        let enc_seed = encrypt(seed, pwd).expect("seed to encrypt ok");
        let enc_seed_t = encrypt(seed, pwd).expect("seed to encrypt ok");

        // check that random IV is correctly applied
        assert_ne!(enc_seed, enc_seed_t);

        let dec_seed = decrypt(&enc_seed, pwd).expect("seed to decrypt ok");

        // check that decryption matches original seed
        assert_eq!(dec_seed, seed);
    }
}
