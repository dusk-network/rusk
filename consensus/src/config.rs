// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Maximum number of steps Consensus could run.
pub const CONSENSUS_MAX_STEP: u8 = 213;
/// Percentage number that determines a quorum.
pub const CONSENSUS_QUORUM_THRESHOLD: f64 = 0.67;
/// Initial step timeout in milliseconds.
pub const CONSENSUS_TIMEOUT_MS: u64 = 5000;
/// Maximum step timeout.
pub const CONSENSUS_MAX_TIMEOUT_MS: u64 = 60 * 1000;
/// Artifical delay on each selection step.
pub const CONSENSUS_DELAY_MS: u64 = 1000;
/// Default number of workers to process agreements.
pub const ACCUMULATOR_WORKERS_AMOUNT: usize = 6;
