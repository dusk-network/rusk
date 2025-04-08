// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`BlockFilter`], a specific [`Filter`] implementation used
//! for WebSocket subscriptions related to block events.
//!
//! This filter is associated with subscriptions like `subscribeBlockAcceptance`
//! and `subscribeBlockFinalization`. Currently, its primary role within the
//! [`Filter::matches`] method is to check if the event is of the expected
//! block-related type.
//!
//! It also carries an `include_txs` flag, typically derived from subscription
//! parameters. This flag signals to the `SubscriptionManager` whether the
//! client requested full transaction details in the notification payload. The
//! flag itself does *not* influence the `matches` logic.
//!
//! Construction is done via the [`BlockFilter::builder()`] method.
//!
//! # Related Modules
//! - [`crate::jsonrpc::infrastructure::subscription::filters`]: Parent module
//!   defining the core [`Filter`] trait.
//! - [`crate::jsonrpc::infrastructure::subscription::manager`]: The
//!   [`SubscriptionManager`] uses filters to route events.

use std::any::Any;
use std::fmt::Debug;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder struct representing the data associated with a block event.
//
// This struct is primarily used within this module for demonstrating and
// testing the `BlockFilter::matches` logic. The actual event type provided by
// the event source (e.g., `node_data::block::Block` or a custom event struct)
// must be downcastable for the filter to function.
///
/// The fields here are examples and not strictly required by the filter logic.
#[derive(Debug, Clone)]
pub struct BlockEventData {
    /// Example field: Block height.
    pub height: u64,
    /// Example field: Indicates if the block contains transactions.
    pub has_transactions: bool,
}

/// A [`Filter`] implementation for block-related subscription events.
//
// This filter checks if an incoming event is of the expected type for block
// subscriptions (e.g., associated with `BlockAcceptance` or
// `BlockFinalization`).
//
// The `include_txs` field determines the desired verbosity of the resulting
// notification payload (whether to include full transaction details) but does
// not affect whether an event `matches` this filter.
///
/// Use the [`BlockFilter::builder()`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filters::{BlockFilter, Filter, BlockEventData};
///
/// // Build a filter (include_txs flag doesn't affect matching)
/// let block_filter = BlockFilter::builder().include_txs(true).build();
///
/// // Create a sample block event
/// let block_event = BlockEventData { height: 123, has_transactions: true };
/// struct NonBlockEvent;
///
/// // The filter matches the correct event type
/// assert!(block_filter.matches(&block_event));
///
/// // The filter does not match other event types
/// assert!(!block_filter.matches(&NonBlockEvent));
///
/// // Accessing the include_txs flag
/// assert!(block_filter.include_txs());
/// ```
#[derive(Debug, Clone)]
pub struct BlockFilter {
    include_txs: bool,
}

impl BlockFilter {
    /// Creates a new builder for constructing a `BlockFilter`.
    ///
    /// Returns a [`BlockFilterBuilder`] with default values (`include_txs` is
    /// false).
    pub fn builder() -> BlockFilterBuilder {
        BlockFilterBuilder::default()
    }

    /// Indicates whether the original subscription requested the inclusion of
    /// full transaction details in event notifications.
    ///
    /// This is used by the `SubscriptionManager` when formatting the event data
    /// to be sent to the client and does not affect the filter's `matches`
    /// logic.
    pub fn include_txs(&self) -> bool {
        self.include_txs
    }
}

impl Filter for BlockFilter {
    /// Checks if a given event is of the expected type for block subscriptions.
    ///
    /// It attempts to downcast the `event` to [`BlockEventData`] (or the actual
    /// expected block event type). If the downcast is successful, it returns
    /// `true`, indicating the event is relevant to this filter. The
    /// `include_txs` state of the filter does *not* influence this check.
    ///
    /// # Returns
    ///
    /// `true` if the event is identified as a block-related event type, `false`
    /// otherwise.
    fn matches(&self, event: &dyn Any) -> bool {
        // Attempt to downcast to the expected concrete event type.
        // Replace `BlockEventData` with the actual event type when known.
        event.downcast_ref::<BlockEventData>().is_some()
    }
}

/// Builder for [`BlockFilter`].
///
/// Provides a fluent interface for constructing a `BlockFilter`, primarily for
/// setting the `include_txs` flag.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filters::BlockFilter;
///
/// let filter_with_txs = BlockFilter::builder().include_txs(true).build();
/// assert!(filter_with_txs.include_txs());
///
/// let filter_without_txs = BlockFilter::builder().include_txs(false).build();
/// assert!(!filter_without_txs.include_txs());
///
/// // Default is false
/// let filter_default = BlockFilter::builder().build();
/// assert!(!filter_default.include_txs());
/// ```
#[derive(Debug, Default)]
pub struct BlockFilterBuilder {
    include_txs: bool,
}

impl BlockFilterBuilder {
    /// Sets whether the subscription requests full transaction details in block
    /// event notifications.
    ///
    /// This controls the verbosity of the payload sent to the client but does
    /// not affect the filtering logic itself.
    ///
    /// Defaults to `false`.
    pub fn include_txs(mut self, include_txs: bool) -> Self {
        self.include_txs = include_txs;
        self
    }

    /// Builds the final [`BlockFilter`].
    pub fn build(self) -> BlockFilter {
        BlockFilter {
            include_txs: self.include_txs,
        }
    }
}
