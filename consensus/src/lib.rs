// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate core;

pub mod commons;
pub mod consensus;
pub mod user;

mod aggregator;
pub mod config;
mod execution_ctx;
mod msg_handler;
pub mod operations;
mod phase;
mod proposal;
pub mod queue;
pub mod quorum;
mod ratification;
mod step_votes_reg;
mod validation;

pub use ratification::step::build_ratification_payload;
pub use validation::step::build_validation_payload;

mod iteration_ctx;
pub mod merkle;

#[cfg(test)]
mod tests {}
