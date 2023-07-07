// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use rand::prelude::*;
use rand::rngs::StdRng;
use tracing::info;

use dusk_bls12_381_sign::SecretKey as BlsSecretKey;
use dusk_pki::{SecretKey, SecretSpendKey};

pub static SSK: LazyLock<SecretSpendKey> = LazyLock::new(|| {
    info!("Generating SecretSpendKey");
    let mut rng = StdRng::seed_from_u64(0xdead);

    SecretSpendKey::random(&mut rng)
});

pub static SK: LazyLock<SecretKey> = LazyLock::new(|| {
    info!("Generating SecretKey");
    let mut rng = StdRng::seed_from_u64(0xdead);

    SecretKey::random(&mut rng)
});

pub static BLS_SK: LazyLock<BlsSecretKey> = LazyLock::new(|| {
    info!("Generating BlsSecretKey");
    let mut rng = StdRng::seed_from_u64(0xdead);

    BlsSecretKey::random(&mut rng)
});
