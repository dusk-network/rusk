// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Rusk JSON-RPC Server Module
//!
//! This module implements the JSON-RPC 2.0 server for Rusk, providing both
//! HTTP and WebSocket interfaces for interacting with the node.

pub mod config;
pub mod error;
pub mod infrastructure;
// pub mod model; // To be added later
pub mod service;
