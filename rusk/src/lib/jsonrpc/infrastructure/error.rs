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
    /// Failed to initialize or connect to the database component.
    #[error("Database component initialization/connection failed: {0}")]
    InitializationFailed(String),
    /// An error occurred while executing a database query.
    #[error("Database query failed: {0}")]
    QueryFailed(String),
    /// The requested data was not found in the database.
    #[error("Data not found in database: {0}")]
    NotFound(String),
    /// An internal error occurred within the database component or adapter.
    #[error("Database internal error: {0}")]
    InternalError(String),
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

/// Errors related to archive operations.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ArchiveError {
    /// Failed to initialize or connect to the archive component.
    #[error("Archive component initialization/connection failed: {0}")]
    InitializationFailed(String),
    /// An error occurred while executing an archive query.
    #[error("Archive query failed: {0}")]
    QueryFailed(String),
    /// The requested data was not found in the archive.
    #[error("Data not found in archive: {0}")]
    NotFound(String),
    /// An internal error occurred within the archive component or adapter.
    #[error("Archive internal error: {0}")]
    InternalError(String),
}

/// Errors related to network operations.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum NetworkError {
    /// An error occurred while executing a network query or operation.
    #[error("Network query failed: {0}")]
    QueryFailed(String),
    /// An internal error occurred within the network component or adapter.
    #[error("Internal network error: {0}")]
    InternalError(String),
    /// The requested network operation timed out.
    #[error("Operation timed out: {0}")]
    Timeout(String),
}

/// Errors related to Virtual Machine (VM) operations.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum VmError {
    /// An error occurred while executing a VM query or operation.
    #[error("VM query failed: {0}")]
    QueryFailed(String),
    /// An internal error occurred within the VM component or adapter.
    #[error("Internal VM error: {0}")]
    InternalError(String),
    /// The VM execution failed.
    #[error("VM execution failed: {0}")]
    ExecutionFailed(String),
}

/// Consolidated error enum for the infrastructure layer.
///
/// This enum wraps specific infrastructure errors, allowing functions within
/// this layer to return a unified error type.
#[derive(Error, Debug)]
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

    /// An archive error occurred.
    #[error("Archive error: {0}")]
    Archive(#[from] ArchiveError),

    /// A network error occurred.
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// A VM error occurred.
    #[error("VM error: {0}")]
    Vm(#[from] VmError),

    /// A subscription management error occurred.
    #[error("Subscription error: {0}")]
    Subscription(
        #[from]
        crate::jsonrpc::infrastructure::subscription::error::SubscriptionError,
    ),
}
