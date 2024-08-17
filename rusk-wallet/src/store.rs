// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::clients::StateStore;
use crate::Error;

use dusk_bytes::{Error as BytesError, Serializable};
use dusk_wallet_core::Store;

#[derive(Clone)]
pub struct Seed([u8; 64]);

impl Default for Seed {
    fn default() -> Self {
        Self([0u8; 64])
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

impl Store for LocalStore {
    type Error = Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; Seed::SIZE], Self::Error> {
        Ok(self.seed.to_bytes())
    }
}

impl Store for StateStore {
    type Error = Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; Seed::SIZE], Self::Error> {
        Ok(self.store.seed.to_bytes())
    }
}

impl LocalStore {
    /// Creates a new store from a known seed
    pub(crate) fn new(seed: Seed) -> Self {
        LocalStore { seed }
    }
}
