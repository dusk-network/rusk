// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Params {
    pub max_inv_entries: usize,
    pub max_ongoing_requests: usize,

    /// delay_on_resp_msg is in milliseconds. It mitigates stress on UDP
    /// buffers when network latency is 0 (localnet network only)
    pub delay_on_resp_msg: Option<u64>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_inv_entries: 100,
            max_ongoing_requests: 1000,
            delay_on_resp_msg: None,
        }
    }
}

impl std::fmt::Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "max_inv_entries: {}, max_ongoing_requests: {}",
            self.max_inv_entries, self.max_ongoing_requests,
        )
    }
}
