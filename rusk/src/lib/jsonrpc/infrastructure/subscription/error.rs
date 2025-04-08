// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error types related to subscription management within the JSON-RPC
//! infrastructure.
//!
//! This module defines the [`SubscriptionError`] enum, which encapsulates
//! various errors that can occur during the lifecycle of WebSocket
//! subscriptions, including creation, management, and event publishing.
//!
//! # Examples
//!
//! ```rust
//! use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
//!
//! fn handle_subscription_request(/* ... */) -> Result<(), SubscriptionError> {
//!     // Simulate an error condition
//!     let topic = "unsupported_chain_events";
//!     Err(SubscriptionError::InvalidTopic(topic.to_string()))
//! }
//!
//! match handle_subscription_request() {
//!     Ok(_) => println!("Subscription successful!"),
//!     Err(e) => {
//!         eprintln!("Subscription failed: {}", e);
//!         match e {
//!             SubscriptionError::InvalidTopic(topic) => {
//!                 // Specific handling for invalid topic
//!                 assert_eq!(topic, "unsupported_chain_events");
//!             }
//!             _ => { /* Handle other errors */ }
//!         }
//!     }
//! }
//! ```

use thiserror::Error;

/// Represents errors that can occur during subscription management.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubscriptionError {
    /// Indicates that the specified topic is not supported or invalid.
    /// This typically occurs when a client tries to subscribe to a topic
    /// that the server does not recognize.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::InvalidTopic("unknown_topic".to_string());
    /// assert_eq!(format!("{}", error), "Invalid subscription topic: unknown_topic");
    /// ```
    #[error("Invalid subscription topic: {0}")]
    InvalidTopic(String),

    /// Indicates that the provided subscription ID is invalid or does not
    /// exist. This can happen when trying to unsubscribe with an ID that
    /// was never issued or has already been removed.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::InvalidSubscription("invalid-sub-id".to_string());
    /// assert_eq!(format!("{}", error), "Invalid subscription ID: invalid-sub-id");
    /// ```
    #[error("Invalid subscription ID: {0}")]
    InvalidSubscription(String),

    /// Indicates that the filter configuration provided for a subscription is
    /// invalid. This might be due to incorrect parameters, unsupported
    /// filter types, or syntax errors in filter definitions.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::InvalidFilter("missing required field".to_string());
    /// assert_eq!(format!("{}", error), "Invalid subscription filter: missing required field");
    /// ```
    #[error("Invalid subscription filter: {0}")]
    InvalidFilter(String),

    /// Indicates that the specified session ID does not exist or is no longer
    /// valid. This usually happens when trying to operate on subscriptions
    /// associated with a disconnected or unknown client session.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::SessionNotFound("unknown-session-id".to_string());
    /// assert_eq!(format!("{}", error), "Session not found: unknown-session-id");
    /// ```
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Indicates a failure during the event publishing process.
    /// This might be due to issues with the underlying communication channel
    /// (e.g., WebSocket closed), serialization problems, or other IO
    /// errors. Contains the reason for the failure.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::PublishFailed("connection closed unexpectedly".to_string());
    /// assert_eq!(format!("{}", error), "Failed to publish event: connection closed unexpectedly");
    /// ```
    #[error("Failed to publish event: {0}")]
    PublishFailed(String),

    /// Indicates that the internal event channel used for asynchronous
    /// publishing is closed. This usually means the background processing
    /// task has stopped or panicked. Contains the reason for the channel
    /// closure.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::ChannelClosed("background task terminated".to_string());
    /// assert_eq!(format!("{}", error), "Event channel closed: background task terminated");
    /// ```
    #[error("Event channel closed: {0}")]
    ChannelClosed(String),

    /// Indicates that the specific topic being published to has been closed or
    /// is inactive. This might occur if a topic is dynamically managed and
    /// has been shut down. Contains the name of the closed topic.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::TopicClosed("mempool_events".to_string());
    /// assert_eq!(format!("{}", error), "Subscription topic closed: mempool_events");
    /// ```
    #[error("Subscription topic closed: {0}")]
    TopicClosed(String),

    /// Indicates that a client has attempted to create too many subscriptions,
    /// exceeding configured limits. Contains information about the limit
    /// that was exceeded.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::TooManySubscriptions("limit of 10 reached".to_string());
    /// assert_eq!(format!("{}", error), "Too many subscriptions: limit of 10 reached");
    /// ```
    #[error("Too many subscriptions: {0}")]
    TooManySubscriptions(String),

    /// Indicates that a string could not be parsed into a valid
    /// `SubscriptionId` because it does not conform to the expected UUID
    /// format. Contains the underlying parsing error message.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::InvalidSubscriptionIdFormat("invalid uuid string".to_string());
    /// assert_eq!(format!("{}", error), "Invalid subscription ID format: invalid uuid string");
    /// ```
    #[error("Invalid subscription ID format: {0}")]
    InvalidSubscriptionIdFormat(String),

    /// Indicates that a string used for a `SessionId` is invalid (e.g.,
    /// empty). Contains a description of the format violation.
    ///
    /// # Example
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
    ///
    /// let error = SubscriptionError::InvalidSessionIdFormat("Session ID cannot be empty".to_string());
    /// assert_eq!(format!("{}", error), "Invalid session ID format: Session ID cannot be empty");
    /// ```
    #[error("Invalid session ID format: {0}")]
    InvalidSessionIdFormat(String),
}
