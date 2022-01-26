// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::CliError;
use bip39::{Language, Mnemonic, MnemonicType, Seed};

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

/// Encrypt the wallet seed
pub(crate) fn encrypt(
    seed: &[u8; 64],
    _pwd: String,
) -> Result<Vec<u8>, CliError> {
    Ok(seed.to_vec())
}

/// Decrypt wallet seed
pub(crate) fn decrypt(
    bytes: Vec<u8>,
    _pwd: String,
) -> Result<Vec<u8>, CliError> {
    Ok(bytes)
}
