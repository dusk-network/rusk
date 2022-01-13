// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;
pub mod services;

use std::env::temp_dir;
use std::path::PathBuf;

pub use common::TestContext;
use lazy_static::lazy_static;
use rand::RngCore;

/// Returns a new random socket path withing `SOCKET_DIR`.
pub fn new_socket_path() -> PathBuf {
    let mut rng = rand::thread_rng();
    SOCKET_DIR
        .join(rng.next_u32().to_string())
        .with_extension("rusk")
}

lazy_static! {
    /// Default UDS directory that will contains UDSs for Rusk's GRPC-server to bind on.
    pub static ref SOCKET_DIR: PathBuf = {
        temp_dir().join(".rusk_test_sockets")
    };
}
