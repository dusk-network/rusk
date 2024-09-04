// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::clients::State;

use dusk_bytes::{Error as BytesError, Serializable};

use wallet_core::keys::{self, RNG_SEED};

#[derive(Clone)]
pub struct Seed(keys::Seed);

impl Default for Seed {
    fn default() -> Self {
        Self([0u8; RNG_SEED])
    }
}

impl Serializable<64> for Seed {
    type Error = BytesError;

    fn from_bytes(buff: &[u8; Seed::SIZE]) -> Result<Self, Self::Error> {
        Ok(Self(*buff))
    }
    fn to_bytes(&self) -> [u8; Seed::SIZE] {
        self.0
    }
}

/// Provides a valid wallet seed to dusk_wallet_core
#[derive(Clone)]
pub(crate) struct LocalStore {
    seed: Seed,
}

impl LocalStore {
    /// Retrieves the seed used to derive keys.
    pub fn get_seed(&self) -> &[u8; Seed::SIZE] {
        &self.seed.0
    }
}

impl From<[u8; Seed::SIZE]> for LocalStore {
    fn from(seed: [u8; Seed::SIZE]) -> Self {
        LocalStore { seed: Seed(seed) }
    }
}

impl State {
    /// Retrieves the seed used to derive keys.
    pub fn get_seed(&self) -> &[u8; Seed::SIZE] {
        self.store().get_seed()
    }
}
