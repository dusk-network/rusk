// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::sync::LazyLock;
use std::time::Duration;

use node_data::message::{MESSAGE_MAX_FAILED_ITERATIONS, MESSAGE_MAX_ITER};

/// Maximum number of iterations Consensus runs per a single round.
pub const CONSENSUS_MAX_ITER: u8 = MESSAGE_MAX_ITER;

/// Total credits of steps committees
pub const PROPOSAL_COMMITTEE_CREDITS: usize = 1;
pub const VALIDATION_COMMITTEE_CREDITS: usize = 64;
pub const RATIFICATION_COMMITTEE_CREDITS: usize = 64;

pub const RELAX_ITERATION_THRESHOLD: u8 = MESSAGE_MAX_FAILED_ITERATIONS;
pub const MAX_NUMBER_OF_TRANSACTIONS: usize = 1_000;
pub const MAX_NUMBER_OF_FAULTS: usize = 100;

pub const MAX_BLOCK_SIZE: usize = 1_024 * 1_024;

/// Emergency mode is enabled after 16 iterations
pub const EMERGENCY_MODE_ITERATION_THRESHOLD: u8 = 16;

pub const MIN_STEP_TIMEOUT: Duration = Duration::from_secs(7);
pub const MAX_STEP_TIMEOUT: Duration = Duration::from_secs(40);
pub const TIMEOUT_INCREASE: Duration = Duration::from_secs(2);

mod default {
    pub const MINIMUM_BLOCK_TIME: u64 = 10;
}

pub static MINIMUM_BLOCK_TIME: LazyLock<u64> = LazyLock::new(|| {
    env::var("RUSK_MINIMUM_BLOCK_TIME")
        .unwrap_or_default()
        .parse()
        .unwrap_or(default::MINIMUM_BLOCK_TIME)
});

/// Maximum allowable round difference for message signature verification and
/// for determining if a consensus message is close enough to the network tip
/// for enqueuing.
/// Controls the range of rounds considered relevant to current operations.
pub const MAX_ROUND_DISTANCE: u64 = 10;

// Returns `floor(value/2) + 1`
pub fn majority(value: usize) -> usize {
    value / 2 + 1
}

// Returns `ceil( value/3*2 )`
pub fn supermajority(value: usize) -> usize {
    let sm = value as f32 / 3.0 * 2.0;
    sm.ceil() as usize
}

/// Returns the quorum of a Ratification committee
pub fn ratification_quorum() -> usize {
    supermajority(RATIFICATION_COMMITTEE_CREDITS)
}

/// Returns the quorum of a Validation committee
pub fn validation_quorum() -> usize {
    supermajority(VALIDATION_COMMITTEE_CREDITS)
}

/// Returns the number of credits beyond the quorum for a Validation committee
pub fn validation_extra() -> usize {
    VALIDATION_COMMITTEE_CREDITS - validation_quorum()
}

/// Returns the number of credits beyond the quorum for a Ratification committee
pub fn ratification_extra() -> usize {
    RATIFICATION_COMMITTEE_CREDITS - ratification_quorum()
}

/// Returns whether the current iteration is an emergency iteration
pub fn is_emergency_iter(iter: u8) -> bool {
    iter >= EMERGENCY_MODE_ITERATION_THRESHOLD
}

/// Returns if the next iteration generator needs to be excluded
pub fn exclude_next_generator(iter: u8) -> bool {
    iter < CONSENSUS_MAX_ITER - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_majorities() {
        assert_eq!(majority(4), 3);
        assert_eq!(majority(11), 6);
        assert_eq!(majority(99), 50);
        assert_eq!(supermajority(3), 2);
        assert_eq!(supermajority(9), 6);
        assert_eq!(supermajority(51), 34);
    }

    #[test]
    fn test_quorums() {
        assert_eq!(majority(VALIDATION_COMMITTEE_CREDITS), 33);
        assert_eq!(validation_quorum(), 43);
        assert_eq!(ratification_quorum(), 43);
        assert_eq!(validation_extra(), 21);
        assert_eq!(ratification_extra(), 21);
    }
}
