// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use rand::prelude::*;
use rand::rngs::StdRng;
use tracing::info;

use execution_core::signatures::bls::SecretKey as BlsSecretKey;

#[allow(dead_code)]
pub static STAKE_SK: LazyLock<BlsSecretKey> = LazyLock::new(|| {
    info!("Generating BlsSecretKey");
    let mut rng = StdRng::seed_from_u64(0xdead);

    BlsSecretKey::random(&mut rng)
});
