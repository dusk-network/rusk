// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};
use std::{fmt::Formatter, time::Duration};

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Params {
    /// Number of pending to be processed transactions
    pub max_queue_size: usize,

    /// Maximum number of transactions that can be accepted/stored in mempool
    pub max_mempool_txn_count: usize,

    /// Interval to check for expired transactions
    pub idle_interval: Duration,

    /// Duration after which a transaction is removed from the mempool
    pub mempool_expiry: Duration,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_mempool_txn_count: 10_000,
            idle_interval: Duration::from_secs(60),
            mempool_expiry: Duration::from_secs(3 * 60 * 60 * 24), // 3 days
        }
    }
}

impl std::fmt::Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "max_queue_size: {}", self.max_queue_size)
    }
}
