// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`TransferFilter`], a specialized filter for the
//! `subscribeContractTransferEvents` WebSocket subscription method.
//!
//! This module provides the necessary structures and logic to filter contract
//! transfer events based on the contract ID and a minimum transfer amount,
//! as specified in the Rusk JSON-RPC WebSocket API documentation.
//!
//! The [`TransferFilter`] struct holds the filtering criteria derived from the
//! client's subscription request parameters (`contract_id`, `min_amount`). It
//! implements the core [`Filter`] trait, providing the `matches` method which
//! the `SubscriptionManager` uses to determine if a published event (expected
//! to be of a type containing transfer details like [`TransferEventData`])
//! should be sent to the subscriber.
//!
//! A type-state builder ([`TransferFilterBuilder`]) ensures that the required
//! `contract_id` is always provided during construction.
//!
//! # Related
//! - [`crate::jsonrpc::infrastructure::subscription::filters::Filter`]: The
//!   core filtering trait.
//! - [`crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager`]:
//!   Uses filters to route events.

use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::jsonrpc::infrastructure::subscription::filters::Filter;

/// Placeholder for the actual data type associated with contract transfer
/// events.
///
/// This is used to demonstrate the downcasting and filtering mechanism in
/// `TransferFilter::matches`. The actual type will depend on the event
/// publishing implementation for topics like `ContractTransferEvents`. It must
/// contain at least the contract ID and the transfer amount for filtering.
///
/// See [`rusk::docs::JSON_RPC_websocket_methods::ContractTransferEvent`] for
/// the expected structure.
#[derive(Debug, Clone)]
pub struct TransferEventData {
    /// The ID of the contract that emitted the event (e.g., token contract).
    pub contract_id: String,
    /// The amount transferred.
    pub amount: u64,
    // Other fields like sender, receiver, memo are ignored by this filter.
}

/// Filter for contract transfer events (`ContractTransferEvents`).
///
/// This filter matches events based on the `contract_id` and optionally a
/// minimum transfer `amount`. The `include_metadata` flag indicates whether the
/// subscription requested detailed metadata in the notification payload, but it
/// does not affect the filtering logic of the `matches` method itself.
///
/// Use the [`TransferFilter::builder()`] to construct instances.
///
/// # Examples
///
/// ```rust
/// use std::any::Any;
/// use rusk::jsonrpc::infrastructure::subscription::filters::{Filter, TransferFilter, TransferEventData};
///
/// // Build a filter for a specific contract, minimum amount 1000, requesting metadata
/// let filter = TransferFilter::builder()
///     .contract_id("token_contract_abc".to_string())
///     .min_amount(Some(1000))
///     .include_metadata(true)
///     .build();
///
/// // Create sample transfer events
/// let event_match = TransferEventData {
///     contract_id: "token_contract_abc".to_string(),
///     amount: 1500,
/// };
/// let event_match_exact = TransferEventData {
///     contract_id: "token_contract_abc".to_string(),
///     amount: 1000,
/// };
/// let event_no_match_amount = TransferEventData {
///     contract_id: "token_contract_abc".to_string(),
///     amount: 500,
/// };
/// let event_no_match_contract = TransferEventData {
///     contract_id: "other_contract".to_string(),
///     amount: 2000,
/// };
/// struct NonTransferEvent;
///
/// // The filter matches the correct event type, contract ID, and amount >= min_amount
/// assert!(filter.matches(&event_match));
/// assert!(filter.matches(&event_match_exact));
///
/// // The filter does not match events below the minimum amount
/// assert!(!filter.matches(&event_no_match_amount));
///
/// // The filter does not match events from other contracts
/// assert!(!filter.matches(&event_no_match_contract));
///
/// // The filter does not match other event types
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

    /// Returns the optional minimum transfer amount required to match.
    pub fn min_amount(&self) -> Option<u64> {
        self.min_amount
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

impl Filter for TransferFilter {
    /// Checks if the event matches the transfer filter criteria.
    ///
    /// It attempts to downcast the event to `TransferEventData`. If successful,
    /// it checks if the event's `contract_id` matches the filter's.
    /// If `min_amount` is set in the filter, it further checks if the event's
    /// `amount` is greater than or equal to the `min_amount`.
    ///
    /// # Returns
    ///
    /// `true` if the event matches the filter criteria, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::jsonrpc::infrastructure::subscription::filters::{Filter, TransferFilter, TransferEventData};
    ///
    /// let filter = TransferFilter::builder()
    ///     .contract_id("token_contract_abc".to_string())
    ///     .min_amount(Some(1000))
    ///     .include_metadata(true)
    ///     .build();
    ///
    /// let event_match = TransferEventData {
    ///     contract_id: "token_contract_abc".to_string(),
    ///     amount: 1500,
    /// };
    /// let event_match_exact = TransferEventData {
    ///     contract_id: "token_contract_abc".to_string(),
    ///     amount: 1000,
    /// };
    /// let event_no_match_amount = TransferEventData {
    ///     contract_id: "token_contract_abc".to_string(),
    ///     amount: 500,
    /// };
    /// let event_no_match_contract = TransferEventData {
    ///     contract_id: "other_contract".to_string(),
    ///     amount: 2000,
    /// };
    /// struct NonTransferEvent;
    ///
    /// // The filter matches the correct event type, contract ID, and amount >= min_amount
    /// assert!(filter.matches(&event_match));
    /// assert!(filter.matches(&event_match_exact));
    ///
    /// // The filter does not match events below the minimum amount
    /// assert!(!filter.matches(&event_no_match_amount));
    ///
    /// // The filter does not match events from other contracts
    /// assert!(!filter.matches(&event_no_match_contract));
    ///
    /// // The filter does not match other event types
    /// assert!(!filter.matches(&NonTransferEvent));
    /// ```
    fn matches(&self, event: &dyn Any) -> bool {
        if let Some(transfer_event) = event.downcast_ref::<TransferEventData>()
        {
            // Check if contract ID matches
            if transfer_event.contract_id != self.contract_id {
                return false;
            }

            // Check minimum amount if specified
            if let Some(min) = self.min_amount {
                if transfer_event.amount < min {
                    return false; // Amount is less than the minimum required
                }
            }

            // Contract ID matches, and amount condition (if any) is met
            true
        } else {
            // Not a TransferEventData type
            false
        }
    }
}

// --- Type-State Builder for TransferFilter ---

/// Type state indicating the required `contract_id` has not been set.
#[derive(Debug, Default)]
pub struct NoContractIdT; // Suffix 'T' to avoid name clash if used in same module scope as ContractFilter
                          // states

/// Type state indicating the required `contract_id` has been set.
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
            include_metadata: false, // Default as per JSON-RPC doc
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

    /// Sets an optional minimum transfer amount to filter by.
    ///
    /// If set, only events with an amount greater than or equal to this value
    /// will match. If `None`, amount is not checked.
    ///
    /// Defaults to `None`.
    ///
    /// # Note on Types
    /// While the `subscribeContractTransferEvents` JSON-RPC method accepts
    /// `min_amount` as an optional numeric *string*, this filter internally
    /// uses `Option<u64>` for efficient comparison. The responsibility of
    /// parsing the JSON string parameter into a `u64` lies with the code
    /// handling the incoming subscription request before constructing this
    /// filter.
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
