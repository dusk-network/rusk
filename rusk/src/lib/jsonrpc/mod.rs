// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Rusk JSON-RPC Server Module
//!
//! This module implements the JSON-RPC 2.0 server for Rusk, providing both
//! HTTP and WebSocket interfaces for interacting with the node.
//!
//! ## Feature Flags
//!
//! The server implementation within this module is gated by the
//! `jsonrpc-server` feature flag. Enabling `jsonrpc-server` also relies on
//! underlying node components accessed via adapters, which in turn depend on
//! other features:
//!
//! - **`chain`**: Required by the [`DatabaseAdapter`] (live state database
//!   access).
//! - **`archive`**: Required by the [`ArchiveAdapter`] (historical/indexed data
//!   access).
//!
//! As defined in `Cargo.toml`, the `jsonrpc-server` feature automatically
//! enables both `chain` and `archive`. Therefore, simply enabling
//! `jsonrpc-server` is sufficient to pull in the necessary dependencies.
//!
//! Attempting to build or run code using adapters directly without the
//! corresponding `chain` or `archive` features enabled may result in
//! compile-time errors due to missing types or trait implementations.
//!
//! # Example `Cargo.toml` configuration (relevant parts):
//! ```toml
//! [features]
//! # ... other features ...
//! chain = ["dep:node", "dep:dusk-consensus", "dep:node-data", "dep:rocksdb"]
//! archive = ["chain", "node/archive", "dusk-core/serde"]
//! jsonrpc-server = ["testwallet", "chain", "archive"]
//! ```

pub mod config;
pub mod error;
pub mod infrastructure;
pub mod model;
pub mod server;
pub mod service;
