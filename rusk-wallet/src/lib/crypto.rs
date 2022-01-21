// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bip39::{Mnemonic, MnemonicType, Language, Seed};

#[derive(Debug)]
pub enum CryptoError {
    InvalidPhrase
}

/// Creates and recovers wallet seed from a 12-word BIP39 mnemonic phrase
pub struct MnemSeed {
    pub phrase: String,
    pub seed: [u8;64],
}

impl MnemSeed {

    /// Create a new random mnemonic & seed with a password
    pub fn new(pwd: String) -> Self {
        // create a new randomly generated mnemonic phrase
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);

        // get the HD wallet seed
        let seed = Seed::new(&mnemonic, pwd.as_str());
        let mut seed_bytes = [0u8;64];
        seed_bytes.copy_from_slice(&seed.as_bytes());

        // create and return new MnemSeed
        let phrase: &str = mnemonic.phrase();
        MnemSeed { phrase: phrase.to_string(), seed: seed_bytes }
    }

    /// Generate a seed given a mnemonic phrase and a password
    pub fn from_phrase(phrase: String, pwd: String) -> Result<Self, CryptoError> {
        // generate mnemmonic from user's phrase
        let mnemonic = Mnemonic::from_phrase(phrase.as_str(), Language::English);
        match mnemonic {
            Ok(m) => {
                // recover the seed
                let seed = Seed::new(&m, pwd.as_str());
                let mut seed_bytes = [0u8;64];
                seed_bytes.copy_from_slice(&seed.as_bytes());

                // return 
                Ok(MnemSeed{
                    phrase: phrase, 
                    seed: seed_bytes,
                })
            },
            Err(_) => {
                Err(CryptoError::InvalidPhrase)
            }
        }
    }

}