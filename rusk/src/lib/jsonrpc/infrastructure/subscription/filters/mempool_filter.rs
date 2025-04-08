// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`MempoolFilter`], a specific [`Filter`] implementation used
//! for WebSocket subscriptions related to mempool events.
//!
//! This filter is designed to be used with subscriptions like
//! `subscribeMempoolAcceptance`. It allows clients to receive notifications
//! only for transactions that potentially involve a specific `contract_id`.
//!
//! If no `contract_id` is provided in the filter, it matches all relevant
//! mempool events. The filter also carries an `include_details` flag, which
//! signals to the subscription manager whether the client requested detailed
//! transaction information in the notification payload; this flag does *not*
//! influence the matching logic itself.
//!
//! Construction is done via the [`MempoolFilter::builder()`] method.
//!
//! # Related Modules
//! - [`crate::jsonrpc::infrastructure::subscription::filters`]: Parent module
//!   defining the core [`Filter`] trait.
//! - [`crate::jsonrpc::infrastructure::subscription::manager`]: The
//!   [`SubscriptionManager`] uses filters to route events.

use std::any::Any;
use std::fmt::Debug;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder struct representing the data associated with a mempool event.
//
// This struct is primarily used within this module for demonstrating and
// testing the `MempoolFilter::matches` logic. The actual event type provided by
// the event source (e.g., representing a transaction entering the mempool) must
// be downcastable to a type that exposes an optional contract ID for the filter
// to function correctly when a specific `contract_id` is provided.
///
/// The fields here represent the minimal information needed *by the filter*.
#[derive(Debug, Clone)]
pub struct MempoolEventData {
    /// Optional ID of the contract involved in the transaction, if any.
    pub contract_id: Option<String>,
    /// Indicates if the original event contains detailed information.
    /// This field is present for demonstration but not used by the filter's
    /// `matches` logic.
    pub has_details: bool,
}

/// A [`Filter`] implementation for mempool-related subscription events.
//
// This filter checks incoming events based on an optional `contract_id`. If a
// `contract_id` is specified in the filter, only events associated with that
// specific contract ID will match. If no `contract_id` is specified, all
// applicable mempool events (specifically, those downcastable to a type like
// [`MempoolEventData`]) will match.
//
// The `include_details` field determines the desired verbosity of the resulting
// notification payload but does not affect whether an event `matches`.
///
/// Use the [`MempoolFilter::builder()`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filters::{Filter, MempoolFilter, MempoolEventData};
///
/// // Filter for a specific contract, requesting details
/// let filter_specific = MempoolFilter::builder()
///     .contract_id(Some("contract_123".to_string()))
///     .include_details(true)
///     .build();
///
/// // Filter for any contract, not requesting details
/// let filter_any = MempoolFilter::builder()
///     .contract_id(None)
///     .include_details(false)
///     .build();
///
/// // Sample events
/// let event_contract_123 = MempoolEventData {
///     contract_id: Some("contract_123".to_string()),
///     has_details: true,
/// };
/// let event_contract_456 = MempoolEventData {
///     contract_id: Some("contract_456".to_string()),
///     has_details: true,
/// };
/// let event_no_contract = MempoolEventData {
///     contract_id: None,
///     has_details: false,
/// };
/// struct NonMempoolEvent;
///
/// // filter_specific matches only events for contract_123
/// assert!(filter_specific.matches(&event_contract_123));
/// assert!(!filter_specific.matches(&event_contract_456));
/// assert!(!filter_specific.matches(&event_no_contract)); // Event has no contract ID
/// assert!(!filter_specific.matches(&NonMempoolEvent)); // Wrong type
///
/// // filter_any matches any MempoolEventData
/// assert!(filter_any.matches(&event_contract_123));
/// assert!(filter_any.matches(&event_contract_456));
/// assert!(filter_any.matches(&event_no_contract));
/// assert!(!filter_any.matches(&NonMempoolEvent)); // Wrong type
///
/// // Accessing properties
/// assert_eq!(filter_specific.contract_id(), Some("contract_123"));
/// assert!(filter_specific.include_details());
/// assert!(filter_any.contract_id().is_none());
/// assert!(!filter_any.include_details());
/// ```
#[derive(Debug, Clone, Default)]
pub struct MempoolFilter {
    contract_id: Option<String>,
    include_details: bool,
}

impl MempoolFilter {
    /// Creates a new builder for constructing a `MempoolFilter`.
    ///
    /// Returns a [`MempoolFilterBuilder`] with default values (no specific
    /// contract ID filter, `include_details` is false).
    pub fn builder() -> MempoolFilterBuilder {
        MempoolFilterBuilder::default()
    }

    /// Returns the specific contract ID this filter targets, if any.
    ///
    /// If `Some(contract_id)`, only events matching this ID will pass the
    /// filter. If `None`, the filter does not discriminate based on contract
    /// ID.
    pub fn contract_id(&self) -> Option<&str> {
        self.contract_id.as_deref()
    }

    /// Indicates whether the original subscription requested the inclusion of
    /// full transaction details in event notifications.
    ///
    /// This is used by the `SubscriptionManager` when formatting the event data
    /// to be sent to the client and does not affect the filter's `matches`
    /// logic.
    pub fn include_details(&self) -> bool {
        self.include_details
    }
}

impl Filter for MempoolFilter {
    /// Checks if a given event matches the criteria of this mempool filter.
    ///
    /// 1. It attempts to downcast the `event` to [`MempoolEventData`]. If the
    ///    downcast fails, it returns `false`.
    /// 2. If the filter has a specific `contract_id` set (`Some(filter_cid)`),
    ///    it checks if the event also has a `contract_id` (`Some(event_cid)`)
    ///    and if `event_cid == filter_cid`. If the event has no contract ID or
    ///    the IDs don't match, it returns `false`.
    /// 3. If the filter does *not* have a specific `contract_id` set (`None`),
    ///    it returns `true` (meaning any event of the correct type matches).
    fn matches(&self, event: &dyn Any) -> bool {
        if let Some(mempool_event) = event.downcast_ref::<MempoolEventData>() {
            match &self.contract_id {
                Some(filter_cid) => {
                    // Filter requires a specific contract ID.
                    // Event must have a contract ID, and it must match.
                    mempool_event
                        .contract_id
                        .as_ref()
                        .map_or(false, |event_cid| event_cid == filter_cid)
                }
                None => {
                    // Filter does not specify a contract ID, so any mempool
                    // event matches.
                    true
                }
            }
        } else {
            // Not a MempoolEventData type
            false
        }
    }
}

// --- Builder for MempoolFilter ---

/// Builder for [`MempoolFilter`].
///
/// Allows optional setting of `contract_id` and `include_details`.
///
/// Start with [`MempoolFilter::builder()`].
#[derive(Debug, Default)]
pub struct MempoolFilterBuilder {
    contract_id: Option<String>,
    include_details: bool,
}

impl MempoolFilterBuilder {
    /// Sets an optional contract ID to filter mempool events by.
    ///
    /// - If `Some(contract_id)` is provided, the built filter will only match
    ///   events associated with this specific contract ID.
    /// - If `None` (the default) is provided, the built filter will match any
    ///   relevant mempool event, regardless of whether it involves a contract
    ///   or which contract it involves.
    pub fn contract_id(mut self, contract_id: Option<String>) -> Self {
        self.contract_id = contract_id;
        self
    }

    /// Sets whether the subscription requests full transaction details in
    /// notifications.
    ///
    /// This controls the verbosity of the payload sent to the client but does
    /// not affect the filtering logic itself.
    ///
    /// Defaults to `false`.
    pub fn include_details(mut self, include_details: bool) -> Self {
        self.include_details = include_details;
        self
    }

    /// Builds the final [`MempoolFilter`].
    pub fn build(self) -> MempoolFilter {
        MempoolFilter {
            contract_id: self.contract_id,
            include_details: self.include_details,
        }
    }
}
