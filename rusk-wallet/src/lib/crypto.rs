// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::CliError;
use bip39::{Language, Mnemonic, MnemonicType, Seed};

use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
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

/// Encrypt the wallet seed using AES-256-CBC
/// Requires the seed and the encryption key
/// Will return the ciphertext and the initialization vector (IV)
pub(crate) fn encrypt_seed(
    seed: &[u8; 64],
    _pwd: String,
) -> Result<(Vec<u8>, Vec<u8>), CliError> {
    // this has to be a fresh random value for each execution
    let iv = rand::thread_rng().gen::<[u8; 16]>();

    let cipher = Aes256Cbc::new_from_slices(&_pwd.as_bytes(), &iv).unwrap();
    let enc = cipher.encrypt_vec(seed);

    Ok((enc.to_vec(), iv.to_vec()))
}

/// Decrypt the wallet seed using AES-256-CBC
/// Requires the ciphertext, the IV and the encryption key
/// Will return the seed in plaintext
pub(crate) fn decrypt_seed(
    bytes: Vec<u8>,
    iv: Vec<u8>,
    _pwd: String,
) -> Result<Vec<u8>, CliError> {
    let cipher = Aes256Cbc::new_from_slices(&_pwd.as_bytes(), &iv).unwrap();
    let dec = cipher.decrypt_vec(&bytes).unwrap();

    Ok(dec.to_vec())
}

#[test]
fn encrypt_and_decrypt() {
    // plaintext must be 64 bytes
    let seed =
        "0001020304050607000102030405060700010203040506070001020304050607"
            .as_bytes();
    let mut buffer = [0u8; 64];
    buffer.copy_from_slice(&seed);

    // password must be 32 bytes
    let pwd = "12345678123456781234567812345678";

    // check that random IV is correctly applied
    let (enc, iv) = encrypt_seed(&buffer, pwd.to_string()).unwrap();
    let (enc_diff, _iv_diff) = encrypt_seed(&buffer, pwd.to_string()).unwrap();
    assert_eq!((enc != enc_diff), true);

    // check that decryption matches original plaintext
    let dec = decrypt_seed(enc, iv, pwd.to_string()).unwrap();
    assert_eq!(dec, seed);
}
