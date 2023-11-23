// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Maximum number of steps Consensus runs per a single round.
pub const CONSENSUS_MAX_STEP: u8 = 213;
/// Maximum number of iterations Consensus runs per a single round.
pub const CONSENSUS_MAX_ITER: u8 = CONSENSUS_MAX_STEP / 3;

/// Percentage number that determines a quorum.
pub const CONSENSUS_QUORUM_THRESHOLD: f64 = 0.67;

/// Percentage number that determines a quorum for NIL voting
pub const CONSENSUS_NILQUORUM_THRESHOLD: f64 = CONSENSUS_QUORUM_THRESHOLD;
    // 1f64 - CONSENSUS_QUORUM_THRESHOLD + 0.01;

/// Initial step timeout in milliseconds.
pub const CONSENSUS_TIMEOUT_MS: u64 = 20 * 1000;

/// Maximum step timeout.
pub const CONSENSUS_MAX_TIMEOUT_MS: u64 = 60 * 1000;

/// Steps committee sizes
pub const SELECTION_COMMITTEE_SIZE: usize = 1;
pub const FIRST_REDUCTION_COMMITTEE_SIZE: usize = 64;
pub const SECOND_REDUCTION_COMMITTEE_SIZE: usize = 64;

/// Artifical delay on each selection step.
pub const CONSENSUS_DELAY_MS: u64 = 1000;

/// Default number of workers to process agreements.
pub const ACCUMULATOR_WORKERS_AMOUNT: usize = 6;
pub const ACCUMULATOR_QUEUE_CAP: usize = 100;

/// Enables aggregated agreements messaging in Agreement loop.
pub const ENABLE_AGGR_AGREEMENT: bool = true;

pub const DEFAULT_BLOCK_GAS_LIMIT: u64 = 5 * 1_000_000_000;
