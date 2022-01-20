// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use kadcast::config::Config as KadcastConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) ipc_method: Option<String>,
    pub(crate) socket: String,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) log_level: String,
    pub(crate) kadcast_test: bool,
    pub(crate) kadcast: KadcastConfig,
}

/// Default UDS path that Rusk GRPC-server will connect to.
pub const SOCKET_PATH: &str = "/tmp/rusk_listener";

/// Default port that Rusk GRPC-server will listen to.
pub(crate) const PORT: &str = "8585";
/// Default host_address that Rusk GRPC-server will listen to.
pub(crate) const HOST_ADDRESS: &str = "127.0.0.1";
/// Default log_level.
pub(crate) const LOG_LEVEL: &str = "info";

impl Default for Config {
    fn default() -> Self {
        Config {
            socket: SOCKET_PATH.to_string(),
            host: HOST_ADDRESS.to_string(),
            port: PORT.to_string(),
            log_level: LOG_LEVEL.to_string(),
            ipc_method: None,
            kadcast: KadcastConfig::default(),
            kadcast_test: false,
        }
    }
}
