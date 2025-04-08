// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`MempoolFilter`], a filter for mempool-related WebSocket
//! subscription methods like `subscribeMempoolAcceptance`.
//!
//! This filter allows clients to optionally specify a `contract_id` to only
//! receive notifications about mempool transactions involving that specific
//! contract.
//!
//! The `include_details` flag determines the level of detail in the resulting
//! notification but does not affect the filtering logic.
//!
//! # Related
//! - [`crate::jsonrpc::infrastructure::subscription::filters::Filter`]: The
//!   core filtering trait.

use std::any::Any;
use std::fmt::Debug;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder for data associated with mempool events.
///
/// Used for demonstrating `MempoolFilter::matches`. The actual event type
/// (e.g., representing a transaction entering the mempool) must expose an
/// optional `contract_id` if filtering by contract is intended.
///
/// See [`rusk::docs::JSON_RPC_websocket_methods::MempoolAcceptanceEvent`] for
/// the expected structure.
#[derive(Debug, Clone)]
pub struct MempoolEventData {
    /// Optional ID of the contract involved in the transaction, if applicable.
    pub contract_id: Option<String>,
    /// Placeholder indicating if detailed info is available (used by filter).
    pub has_details: bool, // Corresponds to include_details filter flag
}

/// Filter for mempool subscription events (`MempoolAcceptance`,
/// `MempoolEvents`).
///
/// Matches events based optionally on `contract_id`. The `include_details` flag
/// indicates whether the subscription requested full transaction details in the
/// notification payload, but it does not affect the filtering logic.
///
/// Use the [`MempoolFilter::builder()`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use std::any::Any;
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
    /// Creates a new builder for `MempoolFilter`.
    pub fn builder() -> MempoolFilterBuilder {
        MempoolFilterBuilder::default()
    }

    /// Returns the optional contract ID this filter targets.
    pub fn contract_id(&self) -> Option<&str> {
        self.contract_id.as_deref()
    }

    /// Returns whether the subscription requested inclusion of full transaction
    /// details.
    ///
    /// Used by `SubscriptionManager` to format the notification payload.
    pub fn include_details(&self) -> bool {
        self.include_details
    }
}

impl Filter for MempoolFilter {
    /// Checks if the event matches the mempool filter criteria.
    ///
    /// It attempts to downcast the event to `MempoolEventData`. If successful,
    /// it checks if the filter's `contract_id` is set. If it is, the event's
    /// `contract_id` must match. If the filter's `contract_id` is `None`, any
    /// `MempoolEventData` event matches.
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
    /// Sets the optional contract ID to filter by.
    ///
    /// If `None` (the default), the filter matches transactions regardless of
    /// contract involvement.
    pub fn contract_id(mut self, contract_id: Option<String>) -> Self {
        self.contract_id = contract_id;
        self
    }

    /// Sets whether the filter should indicate that full transaction details
    /// are requested.
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
