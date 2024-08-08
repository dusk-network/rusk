// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Params {
    /// Number of pending to be processed transactions
    pub max_queue_size: usize,

    /// Maximum number of transactions that can be accepted/stored in mempool
    pub max_mempool_txn_count: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_mempool_txn_count: 10_000,
        }
    }
}

impl std::fmt::Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "max_queue_size: {}", self.max_queue_size)
    }
}
