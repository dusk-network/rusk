// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Params {
    #[serde(default = "default_max_inv_entries")]
    pub max_inv_entries: usize,
    #[serde(default = "default_max_ongoing_requests")]
    pub max_ongoing_requests: usize,
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,

    /// delay_on_resp_msg is in milliseconds. It mitigates stress on UDP
    /// buffers when network latency is 0 (localnet network only)
    pub delay_on_resp_msg: Option<u64>,
}

const fn default_max_inv_entries() -> usize {
    100
}
const fn default_max_ongoing_requests() -> usize {
    1000
}
const fn default_max_queue_size() -> usize {
    1000
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_inv_entries: default_max_inv_entries(),
            max_ongoing_requests: default_max_ongoing_requests(),
            delay_on_resp_msg: None,
            max_queue_size: default_max_queue_size(),
        }
    }
}
