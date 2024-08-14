// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};
use std::{fmt::Formatter, time::Duration};

/// Mempool configuration parameters
pub const DEFAULT_EXPIRY_TIME: Duration = Duration::from_secs(3 * 60 * 60 * 24); /* 3 days */
pub const DEFAULT_IDLE_INTERVAL: Duration = Duration::from_secs(60 * 60); /* 1 hour */

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Params {
    /// Number of pending to be processed transactions
    pub max_queue_size: usize,

    /// Maximum number of transactions that can be accepted/stored in mempool
    pub max_mempool_txn_count: usize,

    /// Interval to check for expired transactions
    #[serde(with = "humantime_serde")]
    pub idle_interval: Option<Duration>,

    /// Duration after which a transaction is removed from the mempool
    #[serde(with = "humantime_serde")]
    pub mempool_expiry: Option<Duration>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_mempool_txn_count: 10_000,
            idle_interval: Some(DEFAULT_IDLE_INTERVAL),
            mempool_expiry: Some(DEFAULT_EXPIRY_TIME),
        }
    }
}

impl std::fmt::Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "max_queue_size: {}, max_mempool_txn_count: {},
         idle_interval: {:?}, mempool_expiry: {:?}",
            self.max_queue_size,
            self.max_mempool_txn_count,
            self.idle_interval,
            self.mempool_expiry
        )
    }
}
