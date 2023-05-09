// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Formatter;

use clap::ArgMatches;
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

impl std::fmt::Display for &Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "max_inv_entries: {}, max_ongoing_requests: {}",
            self.max_inv_entries, self.max_ongoing_requests,
        )
    }
}

impl Params {
    pub fn merge(&mut self, matches: &ArgMatches) {
        if let Some(max_inv_entries) = matches.value_of("max_inv_entries") {
            match max_inv_entries.parse() {
                Ok(max_inv_entries) => {
                    self.max_inv_entries = max_inv_entries;
                }
                Err(e) => {
                    tracing::error!("Failed to parse max_inv_entries: {:?}", e);
                }
            }
        };

        if let Some(max_ongoing_requests) =
            matches.value_of("max_ongoing_requests")
        {
            match max_ongoing_requests.parse() {
                Ok(max_ongoing_requests) => {
                    self.max_ongoing_requests = max_ongoing_requests;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to parse max_ongoing_requests: {:?}",
                        e
                    );
                }
            }
        };

        if let Some(delay_on_resp_msg) = matches.value_of("delay_on_resp_msg") {
            match delay_on_resp_msg.parse() {
                Ok(delay_on_resp_msg) => {
                    self.delay_on_resp_msg = Some(delay_on_resp_msg);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to parse delay_on_resp_msg: {:?}",
                        e
                    );
                }
            }
        };
    }
}
