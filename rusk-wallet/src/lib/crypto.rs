// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bip39::{Language, Mnemonic, MnemonicType, Seed};

use aes::Aes256;
use blake3::Hash;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::lib::SEED_SIZE;
use crate::Error;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Creates and recovers wallet seed from a 12-word BIP39 mnemonic phrase
pub(crate) struct MnemSeed {
    pub phrase: String,
    pub seed: [u8; SEED_SIZE],
}

impl MnemSeed {
    /// Create a new random mnemonic & seed with a password
    pub fn new(pwd: &str) -> Self {
        // create a new randomly generated mnemonic phrase
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);

        // get the HD wallet seed
        let seed = Seed::new(&mnemonic, pwd);
        let mut seed_bytes = [0u8; SEED_SIZE];
        seed_bytes.copy_from_slice(seed.as_bytes());

        // create and return new MnemSeed
        let phrase: &str = mnemonic.phrase();
        MnemSeed {
            phrase: phrase.to_string(),
            seed: seed_bytes,
        }
    }

    /// Generate a seed given a mnemonic phrase and a password
    pub fn from_phrase(phrase: &str, pwd: &str) -> Result<Self, Error> {
        // generate mnemmonic from user's phrase
        let mnemonic = Mnemonic::from_phrase(phrase, Language::English);
        match mnemonic {
            Ok(m) => {
                // recover the seed
                let gen_seed = Seed::new(&m, pwd);
                let mut seed = [0u8; SEED_SIZE];
                seed.copy_from_slice(gen_seed.as_bytes());

                // return
                Ok(MnemSeed {
                    phrase: String::from(phrase),
                    seed,
                })
            }
            Err(_) => Err(Error::InvalidMnemonicPhrase),
        }
    }

    /// Returns true if a given mnemonic phrase is valid
    pub fn is_valid(phrase: &str) -> bool {
        Mnemonic::from_phrase(phrase, Language::English).is_ok()
    }

}

/// Encrypts data using a password.
pub(crate) fn encrypt(plaintext: &[u8], pwd: Hash) -> Result<Vec<u8>, Error> {
    let mut iv = vec![0; 16];
    let mut rng = OsRng::default();
    rng.fill_bytes(&mut iv);

    let cipher = Aes256Cbc::new_from_slices(pwd.as_bytes(), &iv)?;
    let enc = cipher.encrypt_vec(plaintext);

    let ciphertext = iv.into_iter().chain(enc.into_iter()).collect();
    Ok(ciphertext)
}

/// Decrypts data encrypted with `encrypt`.
pub(crate) fn decrypt(ciphertext: &[u8], pwd: Hash) -> Result<Vec<u8>, Error> {
    let iv = &ciphertext[..16];
    let enc = &ciphertext[16..];

    let cipher = Aes256Cbc::new_from_slices(pwd.as_bytes(), iv)?;
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

        let enc_seed = encrypt(seed, pwd).expect("seed to encrypt ok");
        let enc_seed_t = encrypt(seed, pwd).expect("seed to encrypt ok");

        // check that random IV is correctly applied
        assert_ne!(enc_seed, enc_seed_t);

        let dec_seed = decrypt(&enc_seed, pwd).expect("seed to decrypt ok");

        // check that decryption matches original seed
        assert_eq!(dec_seed, seed);
    }
}
