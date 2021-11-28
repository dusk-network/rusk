// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;
pub mod schedule;
pub mod services;

pub use common::TestContext;
use lazy_static::lazy_static;
use std::{env::temp_dir, fs, path::PathBuf};

lazy_static! {
    /// Default UDS path that Rusk GRPC-server will connect to.
    pub static ref SOCKET_PATH: PathBuf = {
        let tmp_dir = temp_dir().join(".rusk").join(".tmp_test");
        fs::create_dir_all(tmp_dir.clone()).expect("Error creating tmp testing dir");
        tmp_dir.join("rusk_listener")
    };
}
