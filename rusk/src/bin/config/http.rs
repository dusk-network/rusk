// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone)]
pub struct HttpConfig {
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    #[serde(default = "default_listen")]
    pub listen: bool,
    listen_address: Option<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            listen_address: None,
            cert: None,
            key: None,
        }
    }
}

const fn default_listen() -> bool {
    true
}

impl HttpConfig {
    pub fn listen_addr(&self) -> String {
        self.listen_address
            .clone()
            .unwrap_or("127.0.0.1:8080".into())
    }

    pub(crate) fn merge(&mut self, args: &Args) {
        // Overwrite config ws-listen-addr
        if let Some(http_listen_addr) = &args.http_listen_addr {
            self.listen_address = Some(http_listen_addr.into());
        }
    }
}
