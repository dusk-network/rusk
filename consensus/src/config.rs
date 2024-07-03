// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

/// Maximum number of iterations Consensus runs per a single round.
pub const CONSENSUS_MAX_ITER: u8 = 255;

/// Percentage number that determines quorums.
pub const SUPERMAJORITY_THRESHOLD: f64 = 0.67;
pub const MAJORITY_THRESHOLD: f64 = 0.5;

/// Total credits of steps committees
pub const PROPOSAL_COMMITTEE_CREDITS: usize = 1;
pub const VALIDATION_COMMITTEE_CREDITS: usize = 64;
pub const RATIFICATION_COMMITTEE_CREDITS: usize = 64;

pub const DEFAULT_BLOCK_GAS_LIMIT: u64 = 5 * 1_000_000_000;

pub const RELAX_ITERATION_THRESHOLD: u8 = 10;

/// Emergency mode is enabled only for the last N iterations
pub const EMERGENCY_MODE_ITERATION_THRESHOLD: u8 = CONSENSUS_MAX_ITER - 50;

pub const MIN_STEP_TIMEOUT: Duration = Duration::from_secs(7);
pub const MAX_STEP_TIMEOUT: Duration = Duration::from_secs(40);
pub const TIMEOUT_INCREASE: Duration = Duration::from_secs(2);
pub const MINIMUM_BLOCK_TIME: u64 = 10;
