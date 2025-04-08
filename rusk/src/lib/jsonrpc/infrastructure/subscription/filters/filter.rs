// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines the core [`Filter`] trait, the central abstraction for event
//! filtering within the JSON-RPC WebSocket subscription system.
//!
//! The subscription system allows clients to subscribe to various topics (e.g.,
//! new blocks, specific contract events). Filters provide a mechanism for
//! clients to specify criteria that events must meet to be delivered to them.
//! This reduces unnecessary network traffic and client-side processing by
//! ensuring only relevant events are sent.
//!
//! # The `Filter` Trait
//!
//! Each specific filter implementation (e.g., [`super::BlockFilter`],
//! [`super::ContractFilter`]) corresponds to the parameters accepted by a
//! particular WebSocket subscription method. These implementations hold the
//! client-specified criteria.
//!
//! The core logic resides in the [`Filter::matches`] method. The
//! `SubscriptionManager` calls this method for each published event against
//! every relevant subscription's filter. If `matches` returns `true`, the event
//! is sent to that subscriber.
//!
//! # Type Handling with `dyn Any`
//!
//! To allow a single filtering mechanism across diverse event types (blocks,
//! transactions, contract logs, etc.), the [`matches`](Filter::matches) method
//! accepts the event as `&dyn Any`. Implementors of the `Filter` trait are
//! responsible for:
//! 1. Attempting to downcast the `&dyn Any` to the concrete event type(s) they
//!    are designed to handle.
//! 2. Applying their specific filtering logic based on the fields of the
//!    downcasted event and the criteria stored within the filter itself.
//! 3. Returning `false` if the downcast fails (indicating the event type is not
//!    relevant to this filter) or if the event does not meet the filter's
//!    criteria.
//!
//! Filters must also be `Debug + Send + Sync + 'static` to be stored and safely
//! used concurrently by the `SubscriptionManager`.
//!
//! # Related Modules
//! - [`crate::jsonrpc::infrastructure::subscription::filters`]: The parent
//!   module containing specific filter implementations.
//! - [`crate::jsonrpc::infrastructure::subscription::manager`]: The
//!   [`SubscriptionManager`] which utilizes `Filter` instances.
//! - [`crate::jsonrpc::infrastructure::subscription::types`]: Defines parameter
//!   types used to construct filters.

use std::any::Any;
use std::fmt::Debug;

/// The core trait for event filtering in WebSocket subscriptions.
///
/// Implementors define the logic for matching published events against specific
/// criteria provided by a client during subscription.
///
/// The [`SubscriptionManager`](crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager)
/// holds instances of types implementing this trait (`Box<dyn Filter>`) and
/// uses their [`matches`] method to decide whether to forward an event to a
/// subscriber.
///
/// # Requirements
///
/// Filters must be `Debug + Send + Sync + 'static` to be stored and shared
/// across threads safely.
///
/// # Type Safety with `dyn Any`
///
/// The [`matches`] method receives the event as `&dyn Any`. This design allows
/// the filtering system to handle various event types generically.
/// Implementations must attempt to downcast the `&dyn Any` to the expected
/// concrete event type(s) relevant to the filter. If the downcast fails,
/// `matches` should return `false`.
///
/// # Example Implementation
///
/// ```rust
/// use std::any::Any;
/// use std::fmt::Debug;
/// use rusk::jsonrpc::infrastructure::subscription::filters::Filter;
///
/// // Define a specific event type
/// #[derive(Debug, Clone)]
/// struct ValueEvent {
///     value: i32,
///     category: String,
/// }
///
/// // Define a filter that matches ValueEvents with a positive value
/// #[derive(Debug)]
/// struct PositiveValueFilter {
///     // Filters might hold criteria, e.g., min_value: i32
/// }
///
/// impl Filter for PositiveValueFilter {
///     fn matches(&self, event: &dyn Any) -> bool {
///         // 1. Attempt to downcast to the relevant event type
///         if let Some(value_event) = event.downcast_ref::<ValueEvent>() {
///             // 2. Apply filter logic
///             value_event.value > 0
///         } else {
///             // 3. Return false if downcast fails (wrong event type)
///             false
///         }
///     }
/// }
///
/// // --- Usage Example (Simplified) ---
/// let filter: Box<dyn Filter> = Box::new(PositiveValueFilter {});
///
/// let event_positive = ValueEvent { value: 10, category: "A".to_string() };
/// let event_negative = ValueEvent { value: -5, category: "B".to_string() };
/// struct OtherEvent; // A different, unrelated event type
///
/// assert!(filter.matches(&event_positive));  // Matches
/// assert!(!filter.matches(&event_negative)); // Does not match (value <= 0)
/// assert!(!filter.matches(&OtherEvent));     // Does not match (wrong type)
/// ```
///
/// # See Also
///
/// - [`super::BlockFilter`], [`super::ContractFilter`],
///   [`super::TransferFilter`], [`super::MempoolFilter`] for concrete examples.
pub trait Filter: Debug + Send + Sync + 'static {
    /// Determines if a given event satisfies the criteria defined by this
    /// filter.
    ///
    /// # Arguments
    ///
    /// * `event`: A dynamically-typed reference (`&dyn Any`) to the event
    ///   object being published by the system.
    ///
    /// # Returns
    ///
    /// - `true` if the event is of the expected type for this filter *and* it
    ///   meets the filter's specific criteria.
    /// - `false` if the event is not of the expected type (downcast fails) or
    ///   if it does not meet the criteria.
    fn matches(&self, event: &dyn Any) -> bool;
}
