// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Errors specific to the JSON-RPC service layer.
//!
//! This module defines errors that represent failures within the business logic
//! of the different JSON-RPC service handlers (e.g., block processing,
//! transaction handling, contract interaction). These are distinct from
//! infrastructure errors (like database connection issues) or general JSON-RPC
//! protocol errors.

use thiserror::Error;

/// Consolidated error enum for the service layer.
///
/// This enum represents errors arising from the core logic of handling
/// JSON-RPC requests related to specific node functionalities.
/// Placeholder variants using `String` are used for now and will be refined
/// with more specific error types as each service is implemented.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    /// An error occurred during block processing or retrieval.
    #[error("Block service error: {0}")]
    Block(String),

    /// An error occurred during transaction processing or validation.
    #[error("Transaction service error: {0}")]
    Transaction(String),

    /// An error occurred during smart contract interaction or execution.
    #[error("Contract service error: {0}")]
    Contract(String),

    /// An error occurred related to network information or peer management.
    #[error("Network service error: {0}")]
    Network(String),

    /// An error occurred within the proving system.
    #[error("Prover service error: {0}")]
    Prover(String),

    /// An error occurred related to managing client subscriptions.
    #[error("Subscription service error: {0}")]
    Subscription(String), /* TODO: Replace String with specific
                           * SubscriptionServiceError type */
}
