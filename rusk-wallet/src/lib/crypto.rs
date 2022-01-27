// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::CliError;
use bip39::{Language, Mnemonic, MnemonicType, Seed};

use aes::Aes256;
use blake3::Hash;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::rngs::OsRng;
use rand::Rng;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

/// Creates and recovers wallet seed from a 12-word BIP39 mnemonic phrase
pub(crate) struct MnemSeed {
    pub phrase: String,
    pub seed: [u8; 64],
}

impl MnemSeed {
    /// Create a new random mnemonic & seed with a password
    pub fn new(pwd: &str) -> Self {
        // create a new randomly generated mnemonic phrase
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);

        // get the HD wallet seed
        let seed = Seed::new(&mnemonic, pwd);
        let mut seed_bytes = [0u8; 64];
        seed_bytes.copy_from_slice(seed.as_bytes());

        // create and return new MnemSeed
        let phrase: &str = mnemonic.phrase();
        MnemSeed {
            phrase: phrase.to_string(),
            seed: seed_bytes,
        }
    }

    /// Generate a seed given a mnemonic phrase and a password
    pub fn from_phrase(phrase: &str, pwd: &str) -> Result<Self, CliError> {
        // generate mnemmonic from user's phrase
        let mnemonic = Mnemonic::from_phrase(phrase, Language::English);
        match mnemonic {
            Ok(m) => {
                // recover the seed
                let gen_seed = Seed::new(&m, pwd);
                let mut seed = [0u8; 64];
                seed.copy_from_slice(gen_seed.as_bytes());

                // return
                Ok(MnemSeed {
                    phrase: String::from(phrase),
                    seed,
                })
            }
            Err(_) => Err(CliError::InvalidPhrase),
        }
    }
}

pub(crate) struct EncryptedSeed {
    seed: [u8; 64],
    enc: [u8; 80],
    iv: [u8; 16],
}

impl EncryptedSeed {
    pub const SIZE: usize = 80 + 16;

    pub fn from_seed(seed: [u8; 64]) -> Self {
        EncryptedSeed {
            seed,
            enc: [0u8; 80],
            iv: [0u8; 16],
        }
    }

    /// Parse incoming byte array containing both `enc` and `iv`
    pub fn from_bytes(bytes: &[u8; Self::SIZE]) -> Self {
        let mut enc = [0u8; 80];
        let mut iv = [0u8; 16];

        enc.copy_from_slice(&bytes[..80]);
        iv.copy_from_slice(&bytes[80..]);

        EncryptedSeed {
            seed: [0u8; 64],
            enc,
            iv,
        }
    }

    /// Returns encrypted seed in a byte array containing `enc` and `iv`
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0; Self::SIZE];
        buf[..80].copy_from_slice(&self.enc);
        buf[80..].copy_from_slice(&self.iv);
        buf
    }

    /// Decrypt the wallet seed using AES-256-CBC
    /// Requires the ciphertext, the IV and the encryption key
    /// Will return the seed in plaintext
    pub fn decrypt(&self, pwd: Hash) -> Result<[u8; 64], CliError> {
        let cipher = Aes256Cbc::new_from_slices(pwd.as_bytes(), &self.iv)?;
        let dec = cipher.decrypt_vec(&self.enc)?;
        let mut seed = [0u8; 64];
        seed.copy_from_slice(&dec);
        Ok(seed)
    }

    /// Encrypt the wallet seed using AES-256-CBC
    /// Requires the seed and the encryption key
    /// Will return the ciphertext and the initialization vector (IV)
    pub fn encrypt(&mut self, pwd: Hash) -> Result<(), CliError> {
        let mut iv = [0u8; 16];
        let mut rng = OsRng::default();
        rng.fill(&mut iv);

        let cipher = Aes256Cbc::new_from_slices(pwd.as_bytes(), &iv)?;
        let enc = cipher.encrypt_vec(&self.seed);
        self.enc.copy_from_slice(&enc);
        self.iv.copy_from_slice(&iv);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EncryptedSeed;

    #[test]
    fn encrypt_and_decrypt() {
        let seed =
            b"0001020304050607000102030405060700010203040506070001020304050607";
        let pwd = blake3::hash("greatpassword".as_bytes());

        // encrypt the same seed twice (separately)
        let mut enc_seed = EncryptedSeed::from_seed(*seed);
        enc_seed.encrypt(pwd).unwrap();
        let mut enc_seed_t = EncryptedSeed::from_seed(*seed);
        enc_seed_t.encrypt(pwd).unwrap();

        // check that random IV is correctly applied
        let enc_bytes = enc_seed.to_bytes();
        let enc_bytes_t = enc_seed_t.to_bytes();
        assert_eq!(enc_bytes != enc_bytes_t, true);

        // check that decryption matches original seed
        let dec = EncryptedSeed::from_bytes(&enc_bytes);
        let dec_seed = dec.decrypt(pwd).unwrap();
        assert_eq!(dec_seed, *seed);
    }
}
