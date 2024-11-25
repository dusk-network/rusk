// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use wallet_core::Seed;

use crate::clients::State;

/// Provides a valid wallet seed to dusk_wallet_core
#[derive(Clone)]
pub(crate) struct LocalStore {
    seed: Seed,
}

impl LocalStore {
    /// Retrieves the seed used to derive keys.
    pub fn get_seed(&self) -> &Seed {
        &self.seed
    }

    pub fn inner_mut(&mut self) -> &mut Seed {
        &mut self.seed
    }
}

impl From<Seed> for LocalStore {
    fn from(seed: Seed) -> Self {
        LocalStore { seed }
    }
}

impl State {
    /// Retrieves the seed used to derive keys.
    pub fn get_seed(&self) -> &Seed {
        self.store().get_seed()
    }
}
