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

// --- Transfer Filter ---

/// Placeholder for the actual data type associated with transfer events.
///
/// Used for demonstrating `TransferFilter::matches`. The actual event might be
/// a specific variant of `ContractEventData` or a dedicated type containing
/// parsed transfer details like sender, receiver, and amount.
#[derive(Debug, Clone)]
pub struct TransferEventData {
    /// The ID of the contract that emitted the event.
    pub contract_id: String,
    /// The amount transferred (as a numeric type for comparison).
    pub amount: u64,
    // Other potential fields like sender, receiver are omitted for filter
    // example
}

/// Filter for contract transfer events (`subscribeContractTransferEvents`).
///
/// Matches events based on `contract_id` and optionally a `min_amount`.
/// The `include_metadata` flag is stored but does not affect `matches` logic.
///
/// Use [`TransferFilter::builder()`] to construct.
///
/// # Examples
///
/// ```rust
/// use std::any::Any;
/// use rusk::jsonrpc::infrastructure::subscription::filters::{TransferFilter, Filter, TransferEventData};
///
/// // Build a filter for a specific contract, minimum amount 1000
/// let filter = TransferFilter::builder()
///     .contract_id("token_contract_abc".to_string())
///     .min_amount(Some(1000))
///     .include_metadata(true)
///     .build();
///
/// // Sample events
/// let event_match = TransferEventData { contract_id: "token_contract_abc".to_string(), amount: 1500 };
/// let event_amount_too_low = TransferEventData { contract_id: "token_contract_abc".to_string(), amount: 500 };
/// let event_wrong_contract = TransferEventData { contract_id: "other_contract".to_string(), amount: 2000 };
/// struct NonTransferEvent;
///
/// assert!(filter.matches(&event_match));
/// assert!(!filter.matches(&event_amount_too_low));
/// assert!(!filter.matches(&event_wrong_contract));
/// assert!(!filter.matches(&NonTransferEvent));
///
/// // Accessing filter properties
/// assert_eq!(filter.contract_id(), "token_contract_abc");
/// assert_eq!(filter.min_amount(), Some(1000));
/// assert!(filter.include_metadata());
/// ```
#[derive(Debug, Clone)]
pub struct TransferFilter {
    contract_id: String,
    min_amount: Option<u64>,
    include_metadata: bool,
}

impl TransferFilter {
    /// Creates a new builder for `TransferFilter` requiring the contract ID.
    pub fn builder() -> TransferFilterBuilder<NoContractIdT> {
        TransferFilterBuilder::new()
    }

    /// Returns the contract ID this filter targets.
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the optional minimum transfer amount to filter by.
    pub fn min_amount(&self) -> Option<u64> {
        self.min_amount
    }

    /// Returns whether the subscription requested inclusion of event metadata.
    pub fn include_metadata(&self) -> bool {
        self.include_metadata
    }
}

impl Filter for TransferFilter {
    /// Checks if the event matches the transfer filter criteria.
    ///
    /// It attempts to downcast the event to `TransferEventData`. If successful,
    /// it checks if the `contract_id` matches.
    /// If `min_amount` is set in the filter, it further checks if the event's
    /// `amount` is greater than or equal to the filter's `min_amount`.
    fn matches(&self, event: &dyn Any) -> bool {
        if let Some(transfer_event) = event.downcast_ref::<TransferEventData>()
        {
            // Check contract ID
            if transfer_event.contract_id != self.contract_id {
                return false;
            }

            // Check minimum amount if specified
            if let Some(min) = self.min_amount {
                return transfer_event.amount >= min;
            }

            // Contract ID matches, and no minimum amount filter was applied
            true
        } else {
            // Not a TransferEventData type
            false
        }
    }
}

// --- Type-State Builder for TransferFilter ---

// Note: Suffix 'T' used to differentiate from ContractFilter builder states if
// they were in the same module scope without a dedicated module.

/// Type state indicating the required `contract_id` has not been set for
/// TransferFilter.
#[derive(Debug, Default)]
pub struct NoContractIdT;
/// Type state indicating the required `contract_id` has been set for
/// TransferFilter.
#[derive(Debug)]
pub struct WithContractIdT(String);

/// Builder for [`TransferFilter`].
///
/// Uses the type-state pattern to ensure the required `contract_id` is
/// provided before `build()` can be called.
///
/// Start with [`TransferFilter::builder()`].
#[derive(Debug)]
pub struct TransferFilterBuilder<State> {
    state: State,
    min_amount: Option<u64>,
    include_metadata: bool,
    _phantom: PhantomData<State>,
}

impl TransferFilterBuilder<NoContractIdT> {
    /// Creates a new builder instance in the `NoContractIdT` state.
    fn new() -> Self {
        Self {
            state: NoContractIdT,
            min_amount: None,
            include_metadata: false,
            _phantom: PhantomData,
        }
    }
}

impl<State> TransferFilterBuilder<State> {
    /// Sets the required contract ID for the filter.
    ///
    /// This transitions the builder state to [`WithContractIdT`].
    pub fn contract_id(
        self,
        contract_id: String,
    ) -> TransferFilterBuilder<WithContractIdT> {
        TransferFilterBuilder {
            state: WithContractIdT(contract_id),
            min_amount: self.min_amount,
            include_metadata: self.include_metadata,
            _phantom: PhantomData,
        }
    }

    /// Sets the optional minimum transfer amount.
    ///
    /// If set, only transfers with an amount greater than or equal to this
    /// value will match.
    ///
    /// Defaults to `None`.
    pub fn min_amount(mut self, min_amount: Option<u64>) -> Self {
        self.min_amount = min_amount;
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

impl TransferFilterBuilder<WithContractIdT> {
    /// Builds the final [`TransferFilter`].
    ///
    /// This method is only available when the required `contract_id` has been
    /// set.
    pub fn build(self) -> TransferFilter {
        TransferFilter {
            contract_id: self.state.0,
            min_amount: self.min_amount,
            include_metadata: self.include_metadata,
        }
    }
}
