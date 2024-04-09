// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TelemetryConfig {
    listen_address: Option<String>,
}

impl TelemetryConfig {
    pub fn listen_addr(&self) -> Option<String> {
        self.listen_address.clone()
    }

    pub(crate) fn merge(&mut self, args: &Args) {
        if let Some(listen_addr) = &args.telemetry_listen_addr {
            self.listen_address = Some(listen_addr.into());
        }
    }
}
