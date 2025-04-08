// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`ContractFilter`], a specific [`Filter`] implementation used
//! for WebSocket subscriptions related to generic contract events.
//!
//! This filter is designed to be used with subscriptions like
//! `subscribeContractEvents`. It allows clients to receive notifications only
//! for events emitted by a specific `contract_id` and, optionally, only for
//! events with names matching a provided list (`event_names`).
//!
//! The filter also carries an `include_metadata` flag, which signals to the
//! `SubscriptionManager` whether the client requested detailed event metadata
//! in the notification payload; this flag does *not* influence the matching
//! logic itself.
//!
//! Construction is done via the [`ContractFilter::builder()`] method, which
//! uses the type-state pattern to ensure the required `contract_id` is
//! provided.
//!
//! # Related Modules
//! - [`crate::jsonrpc::infrastructure::subscription::filters`]: Parent module
//!   defining the core [`Filter`] trait.
//! - [`crate::jsonrpc::infrastructure::subscription::manager`]: The
//!   [`SubscriptionManager`] uses filters to route events.

use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder struct representing the data associated with a generic contract
/// event.
///
/// This struct is primarily used within this module for demonstrating and
/// testing the `ContractFilter::matches` logic. The actual event type provided
/// by the event source (e.g., representing an event emitted by a contract) must
/// be downcastable to a type that exposes both a `contract_id` and an
/// `event_name` for the filter to function correctly.
///
/// The fields here represent the minimal information needed *by the filter*.
#[derive(Debug, Clone)]
pub struct ContractEventData {
    /// The ID of the contract that emitted the event.
    pub contract_id: String,
    /// The name of the event emitted.
    pub event_name: String,
    /// Indicates if metadata is included (example field).
    pub has_metadata: bool,
}

/// A [`Filter`] implementation for generic contract event subscriptions.
///
/// This filter checks incoming events based on a mandatory `contract_id` and an
/// optional list of `event_names`. If `event_names` is specified, only events
/// from the target contract whose name is in the list will match. If
/// `event_names` is `None`, all events from the target contract will match.
///
/// The `include_metadata` field determines the desired verbosity of the
/// resulting notification payload but does not affect whether an event
/// `matches` this filter.
///
/// Use the [`ContractFilter::builder()`] to construct instances. This requires
/// setting the `contract_id`.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filters::{ContractFilter, Filter, ContractEventData};
///
/// // Build a filter for a specific contract and specific event names
/// let filter_specific = ContractFilter::builder()
///     .contract_id("contract_123".to_string())
///     .event_names(Some(vec!["Transfer".to_string(), "Approval".to_string()]))
///     .include_metadata(true)
///     .build();
///
/// // Build a filter for a specific contract, any event name
/// let filter_any_event = ContractFilter::builder()
///     .contract_id("contract_123".to_string())
///     .event_names(None)
///     .include_metadata(false)
///     .build();
///
/// // Create sample contract events
/// let event_transfer = ContractEventData {
///     contract_id: "contract_123".to_string(),
///     event_name: "Transfer".to_string(),
///     has_metadata: true,
/// };
/// let event_approval = ContractEventData {
///     contract_id: "contract_123".to_string(),
///     event_name: "Approval".to_string(),
///     has_metadata: false,
/// };
/// let event_other = ContractEventData {
///     contract_id: "contract_123".to_string(),
///     event_name: "Mint".to_string(), // Not in filter_specific's list
///     has_metadata: true,
/// };
/// let event_other_contract = ContractEventData {
///     contract_id: "contract_456".to_string(), // Different contract
///     event_name: "Transfer".to_string(),
///     has_metadata: false,
/// };
/// struct NonContractEvent;
///
/// // Check filter_specific
/// assert!(filter_specific.matches(&event_transfer));
/// assert!(filter_specific.matches(&event_approval));
/// assert!(!filter_specific.matches(&event_other)); // Event name not listed
/// assert!(!filter_specific.matches(&event_other_contract)); // Wrong contract
/// assert!(!filter_specific.matches(&NonContractEvent)); // Wrong type
///
/// // Check filter_any_event
/// assert!(filter_any_event.matches(&event_transfer));
/// assert!(filter_any_event.matches(&event_approval));
/// assert!(filter_any_event.matches(&event_other)); // Matches any event from contract_123
/// assert!(!filter_any_event.matches(&event_other_contract)); // Wrong contract
/// assert!(!filter_any_event.matches(&NonContractEvent)); // Wrong type
///
/// // Accessing filter properties
/// assert_eq!(filter_specific.contract_id(), "contract_123");
/// assert_eq!(filter_specific.event_names(), Some(&["Transfer".to_string(), "Approval".to_string()][..]));
/// assert!(filter_specific.include_metadata());
/// assert_eq!(filter_any_event.contract_id(), "contract_123");
/// assert!(filter_any_event.event_names().is_none());
/// assert!(!filter_any_event.include_metadata());
/// ```
#[derive(Debug, Clone)]
pub struct ContractFilter {
    contract_id: String,
    event_names: Option<Vec<String>>,
    include_metadata: bool,
}

impl ContractFilter {
    /// Creates a new type-state builder for `ContractFilter`.
    ///
    /// The builder starts in a state requiring the `contract_id` to be set.
    pub fn builder() -> ContractFilterBuilder<NoContractId> {
        ContractFilterBuilder::new()
    }

    /// Returns the contract ID that this filter targets.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the optional list of specific event names to filter by.
    ///
    /// - If `Some(names)`, only events from the target `contract_id` whose name
    ///   is present in the `names` list will match.
    /// - If `None`, all events from the target `contract_id` will match,
    ///   regardless of their name.
    pub fn event_names(&self) -> Option<&[String]> {
        self.event_names.as_deref()
    }

    /// Indicates whether the original subscription requested the inclusion of
    /// detailed event metadata in event notifications.
    ///
    /// This is used by the `SubscriptionManager` when formatting the event data
    /// to be sent to the client and does not affect the filter's `matches`
    /// logic.
    pub fn include_metadata(&self) -> bool {
        self.include_metadata
    }
}

impl Filter for ContractFilter {
    /// Checks if a given event matches the criteria of this contract filter.
    ///
    /// 1. It attempts to downcast the `event` to [`ContractEventData`] (or the
    ///    actual expected contract event type).
    /// 2. If the downcast succeeds, it checks if the event's `contract_id`
    ///    matches the filter's mandatory `contract_id`. If not, it returns
    ///    `false`.
    /// 3. If the contract IDs match and the filter specifies a list of
    ///    `event_names` (`Some(names)`), it checks if the event's `event_name`
    ///    is present in the `names` list. If not, it returns `false`.
    /// 4. If the contract IDs match and the filter does *not* specify
    ///    `event_names` (`None`), it returns `true`.
    /// 5. If the initial downcast fails, it returns `false`.
    fn matches(&self, event: &dyn Any) -> bool {
        if let Some(contract_event) = event.downcast_ref::<ContractEventData>()
        {
            // Check if contract ID matches
            if contract_event.contract_id != self.contract_id {
                return false;
            }

            // Check event names if specified
            if let Some(ref names) = self.event_names {
                return names.contains(&contract_event.event_name);
            }

            // Contract ID matches, and no specific event names were requested
            true
        } else {
            // Not a ContractEventData type
            false
        }
    }
}

// --- Type-State Builder for ContractFilter ---

/// Type state indicating the required `contract_id` has not been set.
#[derive(Debug, Default)]
pub struct NoContractId;
/// Type state indicating the required `contract_id` has been set.
#[derive(Debug)]
pub struct WithContractId(String);

/// Builder for [`ContractFilter`] using the type-state pattern.
///
/// This ensures the mandatory `contract_id` field is set before the filter can
/// be built. Optional fields like `event_names` and `include_metadata` can be
/// set at any point before building.
///
/// Start with [`ContractFilter::builder()`].
#[derive(Debug)]
pub struct ContractFilterBuilder<State> {
    state: State,
    event_names: Option<Vec<String>>,
    include_metadata: bool,
    _phantom: PhantomData<State>, // Ensures State is used
}

impl ContractFilterBuilder<NoContractId> {
    /// Creates a new builder instance in the initial state (`NoContractId`).
    fn new() -> Self {
        Self {
            state: NoContractId,
            event_names: None,
            include_metadata: false,
            _phantom: PhantomData,
        }
    }
}

impl<State> ContractFilterBuilder<State> {
    /// Sets the mandatory contract ID for the filter.
    ///
    /// This transitions the builder into the [`WithContractId`] state, allowing
    /// `build()` to be called.
    pub fn contract_id(
        self,
        contract_id: String,
    ) -> ContractFilterBuilder<WithContractId> {
        ContractFilterBuilder {
            state: WithContractId(contract_id),
            event_names: self.event_names,
            include_metadata: self.include_metadata,
            _phantom: PhantomData,
        }
    }

    /// Sets an optional list of specific event names to filter by.
    ///
    /// - If `Some(names)` is provided, the built filter will only match events
    ///   from the specified contract whose name is in the `names` list.
    /// - If `None` (the default) is provided, the built filter will match any
    ///   event from the specified contract, regardless of its name.
    ///
    /// Defaults to `None`.
    pub fn event_names(mut self, event_names: Option<Vec<String>>) -> Self {
        self.event_names = event_names;
        self
    }

    /// Sets whether the subscription requests detailed metadata in event
    /// notifications.
    ///
    /// This controls the verbosity of the payload sent to the client but does
    /// not affect the filtering logic itself.
    ///
    /// Defaults to `false`.
    pub fn include_metadata(mut self, include_metadata: bool) -> Self {
        self.include_metadata = include_metadata;
        self
    }
}

impl ContractFilterBuilder<WithContractId> {
    /// Builds the final [`ContractFilter`].
    ///
    /// This method is only available once the mandatory `contract_id` has been
    /// provided via the [`contract_id`](ContractFilterBuilder::contract_id)
    /// method.
    pub fn build(self) -> ContractFilter {
        ContractFilter {
            contract_id: self.state.0,
            event_names: self.event_names,
            include_metadata: self.include_metadata,
        }
    }
}
