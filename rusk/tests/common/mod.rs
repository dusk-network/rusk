// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod block;
pub mod keys;
pub mod state;
pub mod wallet;

use tracing_subscriber::EnvFilter;

pub fn logger() {
    // Can't use `with_default_env` since we want to have a default
    // directive, and *then* apply the environment variable on top of it,
    // not the other way around.
    let directive = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "rusk=info,tests=info".to_string());

    let filter = EnvFilter::new(directive);
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
