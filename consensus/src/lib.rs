// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

pub mod commons;
pub mod consensus;
pub mod errors;
pub mod user;

mod aggregator;
pub mod config;
mod execution_ctx;
mod msg_handler;
pub mod operations;
mod proposal;
pub mod queue;
pub mod quorum;
mod ratification;
mod step;
mod step_votes_reg;
mod validation;

pub use ratification::step::build_ratification_payload;
pub use validation::step::{build_validation_payload, validate_blob_sidecars};

mod iteration_ctx;
pub mod merkle;

#[cfg(test)]
mod tests {
    // Dev-dependencies only used in integration tests trigger the
    // unused_crate_dependencies lint, so we re-import them here.
    use {criterion as _, rand as _};
}
