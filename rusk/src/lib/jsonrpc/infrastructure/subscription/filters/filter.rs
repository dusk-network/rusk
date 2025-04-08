// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines the core [`Filter`] trait for event filtering
//! within the subscription system.
//!
//! [`Filter`] allows clients to receive only the subset of events they are
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
/// use rusk::jsonrpc::infrastructure::subscription::filters::Filter;
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
