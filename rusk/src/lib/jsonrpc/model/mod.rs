// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Rusk JSON-RPC API Models
//!
//! This module defines the core data structures used for serialization and
//! deserialization within the Rusk JSON-RPC API.
//!
//! ## Overview
//!
//! The primary purpose of this module and its submodules is to provide Rust
//! representations of the JSON objects described in the Rusk JSON-RPC
//! specification. These models act as the boundary layer between the internal
//! Rusk node data types and the external JSON format consumed by clients.
//!
//! ## Key Features:
//!
//! - **Specification Alignment:** Structures are designed to closely match the
//!   fields and formats specified in the official JSON-RPC documentation.
//! - **Serialization/Deserialization:** All models derive `serde::Serialize`
//!   and `serde::Deserialize` to enable straightforward conversion to and from
//!   JSON.
//! - **Data Conversion:** Many models provide `From` implementations to
//!   facilitate conversion from internal `node_data` types (e.g.,
//!   `node_data::ledger::Block`) into the corresponding RPC model (e.g.,
//!   `model::block::Block`).
//! - **Custom Serialization:** The `serde_helper` submodule provides utilities
//!   for custom serialization logic where needed (e.g., serializing large `u64`
//!   values as JSON strings).
//! - **Modularity:** Models are organized into submodules based on their domain
//!   (e.g., `block`, `transaction`, `consensus`).
//!
//! ## Submodules:
//!
//! - [`archive`]: Models for archive-related types.
//! - [`block`]: Models related to blocks, headers, status, and faults.
//! - [`chain`]: Models for overall blockchain statistics.
//! - [`consensus`]: Models representing consensus outcomes (e.g., validation
//!   results).
//! - [`contract`]: Placeholder for contract-related models (if any).
//! - [`gas`]: Models for gas price information.
//! - [`key`]: Models for key-related types.
//! - [`mempool`]: Models for mempool information.
//! - [`network`]: Models for network peer metrics.
//! - [`common`]: Types shared across multiple modules.
//! - [`prover`]: Placeholder for prover-related models (if any).
//! - [`provisioner`]: Models related to provisioner information (stakes, etc.).
//! - [`serde_helper`]: Utility functions for custom `serde` serialization.
//! - [`subscription`]: Placeholder for WebSocket subscription models (if any).
//! - [`transaction`]: Models for transactions, status, types, events, and
//!   simulation results.

pub mod archive;
pub mod block;
pub mod chain;
pub mod common;
pub mod consensus;
pub mod contract;
pub mod gas;
pub mod key;
pub mod mempool;
pub mod network;
pub mod prover;
pub mod provisioner;
pub mod serde_helper;
pub mod subscription;
pub mod transaction;
