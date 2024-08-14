// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

/// Maximum number of iterations Consensus runs per a single round.
pub const CONSENSUS_MAX_ITER: u8 = 50;

/// Percentage number that determines quorums.
pub const SUPERMAJORITY_THRESHOLD: f64 = 0.67;
pub const MAJORITY_THRESHOLD: f64 = 0.5;

/// Total credits of steps committees
pub const PROPOSAL_COMMITTEE_CREDITS: usize = 1;
pub const VALIDATION_COMMITTEE_CREDITS: usize = 64;
pub const VALIDATION_COMMITTEE_QUORUM: f64 =
    VALIDATION_COMMITTEE_CREDITS as f64 * SUPERMAJORITY_THRESHOLD;

pub const RATIFICATION_COMMITTEE_CREDITS: usize = 64;
pub const RATIFICATION_COMMITTEE_QUORUM: f64 =
    RATIFICATION_COMMITTEE_CREDITS as f64 * SUPERMAJORITY_THRESHOLD;

pub const RELAX_ITERATION_THRESHOLD: u8 = 8;

/// Emergency mode is enabled after 16 iterations
pub const EMERGENCY_MODE_ITERATION_THRESHOLD: u8 = 16;

pub const MIN_STEP_TIMEOUT: Duration = Duration::from_secs(7);
pub const MAX_STEP_TIMEOUT: Duration = Duration::from_secs(40);
pub const TIMEOUT_INCREASE: Duration = Duration::from_secs(2);
pub const MINIMUM_BLOCK_TIME: u64 = 10;

/// Returns delta between full quorum and super_majority
pub fn validation_extra() -> usize {
    VALIDATION_COMMITTEE_CREDITS - validation_committee_quorum()
}

pub fn ratification_extra() -> usize {
    RATIFICATION_COMMITTEE_CREDITS - ratification_committee_quorum()
}

/// Returns ceil of RATIFICATION_COMMITTEE_QUORUM
pub fn ratification_committee_quorum() -> usize {
    RATIFICATION_COMMITTEE_QUORUM.ceil() as usize
}

/// Returns ceil of VALIDATION_COMMITTEE_QUORUM
pub fn validation_committee_quorum() -> usize {
    VALIDATION_COMMITTEE_QUORUM.ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_quorum_consts() {
        assert_eq!(validation_committee_quorum(), 43);
        assert_eq!(ratification_committee_quorum(), 43);
        assert_eq!(validation_extra(), 21);
        assert_eq!(ratification_extra(), 21);
    }
}
