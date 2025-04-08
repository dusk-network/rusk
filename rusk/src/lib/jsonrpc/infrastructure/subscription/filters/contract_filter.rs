// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines the [`ContractFilter`] implementation of the core [`Filter`] trait.
//!
//! [`ContractFilter`] allows clients to receive only the subset of events they
//! are interested in for a given topic, reducing network traffic and
//! client-side processing.

use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder for the actual data type associated with contract-related
/// events.
///
/// This is used to demonstrate the downcasting and filtering mechanism in
/// `ContractFilter::matches`. The actual type will depend on the event
/// publishing implementation for topics like `ContractEvents`. It might include
/// fields like the contract ID, event name, parameters, and metadata.
#[derive(Debug, Clone)]
pub struct ContractEventData {
    /// The ID of the contract that emitted the event.
    pub contract_id: String,
    /// The name of the event emitted.
    pub event_name: String,
    /// Indicates if metadata is included (example field).
    pub has_metadata: bool,
}

/// Filter for contract-related subscription events (`ContractEvents`).
///
/// This filter matches events based on the `contract_id` and optionally a list
/// of `event_names`. The `include_metadata` flag indicates whether the
/// subscription requested detailed metadata in the notification payload, but it
/// does not affect the filtering logic of the `matches` method itself.
///
/// Use the [`ContractFilter::builder()`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use std::any::Any;
/// use rusk::jsonrpc::infrastructure::subscription::filters::{ContractFilter, Filter, ContractEventData};
///
/// // Build a filter for a specific contract, any event, requesting metadata
/// let filter = ContractFilter::builder()
///     .contract_id("contract_123".to_string())
///     .include_metadata(true)
///     .build();
///
/// // Create sample contract events
/// let event1 = ContractEventData {
///     contract_id: "contract_123".to_string(),
///     event_name: "Transfer".to_string(),
///     has_metadata: true,
/// };
/// let event2 = ContractEventData {
///     contract_id: "contract_456".to_string(), // Different contract
///     event_name: "Approval".to_string(),
///     has_metadata: false,
/// };
/// struct NonContractEvent;
///
/// // The filter matches the correct event type and contract ID
/// assert!(filter.matches(&event1));
///
/// // The filter does not match events from other contracts
/// assert!(!filter.matches(&event2));
///
/// // The filter does not match other event types
/// assert!(!filter.matches(&NonContractEvent));
///
/// // Accessing filter properties
/// assert_eq!(filter.contract_id(), "contract_123");
/// assert!(filter.event_names().is_none());
/// assert!(filter.include_metadata());
/// ```
#[derive(Debug, Clone)]
pub struct ContractFilter {
    contract_id: String,
    event_names: Option<Vec<String>>,
    include_metadata: bool,
}

impl ContractFilter {
    /// Creates a new builder for `ContractFilter` requiring the contract ID.
    pub fn builder() -> ContractFilterBuilder<NoContractId> {
        ContractFilterBuilder::new()
    }

    /// Returns the contract ID this filter targets.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the optional list of specific event names to filter by.
    ///
    /// If `None`, all event names for the target `contract_id` match.
    pub fn event_names(&self) -> Option<&[String]> {
        self.event_names.as_deref()
    }

    /// Returns whether the subscription requested inclusion of event metadata.
    ///
    /// This value is typically used by the `SubscriptionManager` during event
    /// publication to format the notification payload, not for filtering events
    /// themselves via the `matches` method.
    pub fn include_metadata(&self) -> bool {
        self.include_metadata
    }
}

impl Filter for ContractFilter {
    /// Checks if the event matches the contract filter criteria.
    ///
    /// It attempts to downcast the event to `ContractEventData`. If successful,
    /// it checks if the event's `contract_id` matches the filter's.
    /// If `event_names` is set in the filter, it further checks if the event's
    /// `event_name` is present in the list.
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

/// Builder for [`ContractFilter`].
///
/// Uses the type-state pattern to ensure the required `contract_id` is
/// provided before `build()` can be called.
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
    /// Creates a new builder instance in the `NoContractId` state.
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
    /// Sets the required contract ID for the filter.
    ///
    /// This transitions the builder state to [`WithContractId`].
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
    /// If not set or set to `None`, the filter will match any event name from
    /// the specified contract.
    ///
    /// Defaults to `None`.
    pub fn event_names(mut self, event_names: Option<Vec<String>>) -> Self {
        self.event_names = event_names;
        self
    }

    /// Sets whether the filter should indicate that event metadata is requested
    /// for notifications.
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
    /// This method is only available when the required `contract_id` has been
    /// set.
    pub fn build(self) -> ContractFilter {
        ContractFilter {
            contract_id: self.state.0,
            event_names: self.event_names,
            include_metadata: self.include_metadata,
        }
    }
}
