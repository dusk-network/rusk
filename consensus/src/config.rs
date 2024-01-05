// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

/// Maximum number of iterations Consensus runs per a single round.
pub const CONSENSUS_MAX_ITER: u8 = 255;

/// Number of consecutive attested blocks needed to consider a final block.
pub const CONSENSUS_ROLLING_FINALITY_THRESHOLD: u64 = 5;

/// Percentage number that determines a quorum.
pub const CONSENSUS_QUORUM_THRESHOLD: f64 = 0.67;

/// Initial step timeout.
pub const CONSENSUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum step timeout.
pub const CONSENSUS_MAX_TIMEOUT: Duration = Duration::from_secs(60);

/// Steps committee sizes
pub const PROPOSAL_COMMITTEE_SIZE: usize = 1;
pub const VALIDATION_COMMITTEE_SIZE: usize = 64;
pub const RATIFICATION_COMMITTEE_SIZE: usize = 64;

/// Artifical delay on each Proposal step.
pub const CONSENSUS_DELAY_MS: u64 = 1000;

pub const DEFAULT_BLOCK_GAS_LIMIT: u64 = 5 * 1_000_000_000;

pub const RELAX_ITERATION_THRESHOLD: u8 = 10;
