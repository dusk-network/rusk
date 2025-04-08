// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! JSON-RPC Infrastructure Layer
//!
//! This module contains components responsible for interacting with lower-level
//! systems like the database, state management, rate limiting, and metrics.
//!
//! ## Database Interaction (`db` module)
//!
//! A key component is the `db::DatabaseAdapter` trait, which defines the
//! interface for accessing blockchain data. Instead of directly using the
//! database handle provided by the core `RuskNode`, this dedicated adapter is
//! employed to decouple the JSON-RPC layer from the core node's internal
//! database structure. This approach enhances testability (by allowing easier
//! mocking) and provides flexibility for future changes in either the core
//! database or the JSON-RPC API requirements.

pub mod client_info;
pub mod db;
pub mod error;
pub mod manual_limiter;
pub mod metrics;
pub mod state;
pub mod subscription;
