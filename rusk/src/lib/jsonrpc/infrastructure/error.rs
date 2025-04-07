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

//! Errors specific to the JSON-RPC infrastructure layer.
//!
//! This module defines errors that can occur within the core infrastructure
//! components supporting the JSON-RPC service, such as database interactions,
//! state management, rate limiting, and subscription handling. These errors are
//! distinct from service-level logic errors or general JSON-RPC protocol
//! errors.

use thiserror::Error;

/// Errors related to database operations.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DbError {
    /// Failed to establish a connection to the database.
    #[error("Database connection failed: {0}")]
    Connection(String),
    /// An error occurred while executing a database query.
    #[error("Database query failed: {0}")]
    QueryFailed(String),
    /// The requested data was not found in the database.
    #[error("Data not found in database: {0}")]
    NotFound(String),
    /// An error occurred during database schema migration.
    #[error("Database migration failed: {0}")]
    MigrationFailed(String),
}

/// Errors related to managing shared application state.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum StateError {
    /// The application state was found to be inconsistent.
    #[error("Inconsistent application state: {0}")]
    Inconsistent(String),
    /// Failed to write to the application state.
    #[error("Failed to write state: {0}")]
    WriteFailed(String),
    /// Failed to read from the application state.
    #[error("Failed to read state: {0}")]
    ReadFailed(String),
    /// Failed to acquire a necessary lock for state access.
    #[error("Could not acquire state lock: {0}")]
    LockUnavailable(String),
}

/// Errors related to rate limiting enforcement.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum RateLimitError {
    /// The client has exceeded the allowed request rate.
    #[error("Rate limit exceeded: {0}")]
    LimitExceeded(String),
    /// The rate limiting configuration is invalid.
    #[error("Invalid rate limit configuration: {0}")]
    InvalidConfig(String),
    /// The client has exceeded the manual WebSocket connection rate limit.
    #[error("Manual WebSocket rate limit exceeded: {0}")]
    ManualWebSocketLimitExceeded(String),
    /// The client has exceeded the manual rate limit for a specific method
    /// pattern.
    #[error("Manual method rate limit exceeded: {0}")]
    ManualMethodLimitExceeded(String),
}

/// Errors related to managing WebSocket subscriptions.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum SubscriptionError {
    /// The maximum number of allowed subscriptions has been reached.
    #[error("Subscription limit reached: {0}")]
    LimitReached(String),
    /// Failed to send an update to a subscriber.
    #[error("Failed to send subscription update: {0}")]
    SendFailed(String),
    /// The specified subscription ID was not found.
    #[error("Subscription not found: {0}")]
    NotFound(String),
    /// The communication channel for the subscription was closed.
    #[error("Subscription channel closed: {0}")]
    ChannelClosed(String),
}

/// Consolidated error enum for the infrastructure layer.
///
/// This enum wraps specific infrastructure errors, allowing functions within
/// this layer to return a unified error type.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    /// A database error occurred.
    #[error("Database error: {0}")]
    Database(#[from] DbError),

    /// A state management error occurred.
    #[error("State error: {0}")]
    State(#[from] StateError),

    /// A rate limiting error occurred.
    #[error("Rate limit error: {0}")]
    RateLimit(#[from] RateLimitError),

    /// A subscription management error occurred.
    #[error("Subscription error: {0}")]
    Subscription(#[from] SubscriptionError),
}
