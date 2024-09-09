// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use rand::prelude::*;
use rand::rngs::StdRng;
use tracing::info;

use dusk_bytes::Serializable;
use execution_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};

#[allow(dead_code)]
pub static STAKE_SK: LazyLock<BlsSecretKey> = LazyLock::new(|| {
    info!("Generating BlsSecretKey");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let sk = BlsSecretKey::random(&mut rng);
    let pk = BlsPublicKey::from(&sk);
    info!(
        "Generated BlsSecretKey for BlsPublicKey {}",
        bs58::encode(pk.to_bytes()).into_string()
    );
    sk
});
