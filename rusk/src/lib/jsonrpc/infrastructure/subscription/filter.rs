// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines the core `Filter` trait and implementations for event filtering
//! within the subscription system.
//!
//! Filters allow clients to receive only the subset of events they are
//! interested in for a given topic, reducing network traffic and client-side
//! processing.

use std::any::Any;
use std::fmt::Debug;

/// A trait for defining event filters used in WebSocket subscriptions.
///
/// Implementors of this trait specify criteria for matching events. The
/// `SubscriptionManager` uses these filters to determine if a published event
/// should be sent to a particular subscriber.
///
/// Filters must be `Send + Sync + 'static` to be stored and used across
/// threads safely, particularly within the `SubscriptionManager`'s background
/// task.
///
/// # Type Safety and `Any`
///
/// The `matches` method takes `&dyn Any` as the event type. This allows a
/// single filter mechanism to work with various event types published across
/// different topics. Implementors are responsible for downcasting the `&dyn
/// Any` argument to the concrete event type they expect for their specific
/// topic before applying filter logic. If the downcast fails, the filter should
/// typically return `false` (event doesn't match).
///
/// # Example: Implementing a Simple Filter
///
/// ```rust
/// use std::any::Any;
/// use std::fmt::Debug;
/// // Assuming Filter trait is defined in the current scope or imported
/// # pub trait Filter: Debug + Send + Sync + 'static {
/// #     fn matches(&self, event: &dyn Any) -> bool;
/// # }
///
/// // Example event type
/// #[derive(Debug, Clone)]
/// struct MyEvent {
///     value: i32,
/// }
///
/// // Filter that only matches events with a positive value
/// #[derive(Debug)]
/// struct PositiveValueFilter;
///
/// impl Filter for PositiveValueFilter {
///     fn matches(&self, event: &dyn Any) -> bool {
///         // Attempt to downcast the Any type to our expected event type
///         if let Some(my_event) = event.downcast_ref::<MyEvent>() {
///             // Apply filter logic
///             my_event.value > 0
///         } else {
///             // If it's not the type we expect, it doesn't match
///             false
///         }
///     }
/// }
///
/// let filter = PositiveValueFilter;
/// let positive_event = MyEvent { value: 10 };
/// let negative_event = MyEvent { value: -5 };
/// let zero_event = MyEvent { value: 0 };
/// struct OtherEvent; // A completely different type
///
/// assert!(filter.matches(&positive_event));
/// assert!(!filter.matches(&negative_event));
/// assert!(!filter.matches(&zero_event));
/// assert!(!filter.matches(&OtherEvent)); // Doesn't match different types
/// ```
///
/// # Extending Filters
///
/// Specific filter implementations (like `BlockFilter`, `ContractFilter`, etc.)
/// will typically be derived from the client-provided subscription parameters
/// ([`crate::jsonrpc::infrastructure::subscription::types::BlockSubscriptionParams`], etc.)
/// and will contain the logic specific to their respective event types.
pub trait Filter: Debug + Send + Sync + 'static {
    /// Checks if the given event matches the criteria defined by this filter.
    ///
    /// # Arguments
    ///
    /// * `event`: A dynamic reference to the event object being published.
    ///   Implementors should attempt to downcast this to the expected concrete
    ///   event type.
    ///
    /// # Returns
    ///
    /// `true` if the event matches the filter's criteria, `false` otherwise,
    /// or if the event type is not applicable to this filter.
    fn matches(&self, event: &dyn Any) -> bool;
}

// --- Block Filter ---

/// Placeholder for the actual data type associated with block-related events.
///
/// This is used demonstrate the downcasting mechanism in
/// `BlockFilter::matches`. The actual type will depend on the event publishing
/// implementation for topics like `BlockAcceptance` and `BlockFinalization`. It
/// might be something like `node_data::ledger::Block` or a custom event
/// structure.
#[derive(Debug, Clone)]
pub struct BlockEventData {
    /// Example field: Block height
    pub height: u64,
    /// Example field: Indicates if the block contains transactions.
    pub has_transactions: bool,
}

/// Filter for block-related subscription events (`BlockAcceptance`,
/// `BlockFinalization`).
///
/// This filter currently primarily serves to indicate association with block
/// topics. The `include_txs` field, derived from
/// [`crate::jsonrpc::infrastructure::subscription::types::BlockSubscriptionParams`],
/// is stored but not used in the `matches` logic itself. It's intended to be
/// used later by the `SubscriptionManager` when formatting the notification
/// payload to decide whether to include full transaction details.
///
/// # Examples
///
/// ```rust
/// use std::any::Any;
/// use rusk::jsonrpc::infrastructure::subscription::filter::{BlockFilter, Filter, BlockEventData};
///
/// // Build a filter (e.g., indicating transaction inclusion is desired)
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
    /// Creates a new builder for `BlockFilter`.
    pub fn builder() -> BlockFilterBuilder {
        BlockFilterBuilder::default()
    }

    /// Returns whether the subscription requested inclusion of transaction
    /// data.
    ///
    /// This value is typically used by the `SubscriptionManager` during event
    /// publication to format the notification payload, not for filtering events
    /// themselves via the `matches` method.
    pub fn include_txs(&self) -> bool {
        self.include_txs
    }
}

impl Filter for BlockFilter {
    /// Checks if the event is relevant to block subscriptions.
    ///
    /// It attempts to downcast the event to the expected block event type
    /// (`BlockEventData` placeholder). If successful, it returns `true`,
    /// indicating the event is of the correct type for this filter. It does
    /// *not* filter based on the `include_txs` flag.
    fn matches(&self, event: &dyn Any) -> bool {
        // Attempt to downcast to the expected concrete event type.
        // Replace `BlockEventData` with the actual event type when known.
        event.downcast_ref::<BlockEventData>().is_some()
    }
}

/// Builder for [`BlockFilter`].
///
/// Provides a fluent interface for constructing a `BlockFilter`.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filter::BlockFilter;
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
    /// Sets whether the filter should indicate that full transaction data is
    /// requested for block event notifications.
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
