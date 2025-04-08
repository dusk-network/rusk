// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements the [`TransferFilter`], a specialized [`Filter`] implementation
//! used for WebSocket subscriptions related to contract transfer events.
//!
//! This filter is designed to be used with subscriptions like
//! `subscribeContractTransferEvents`. It allows clients to receive
//! notifications only for transfer events emitted by a specific `contract_id`
//! (e.g., a token contract) and, optionally, only for transfers where the
//! `amount` meets a specified minimum.
//!
//! The filter also carries an `include_metadata` flag, which signals to the
//! `SubscriptionManager` whether the client requested detailed event metadata
//! in the notification payload; this flag does *not* influence the matching
//! logic itself.
//!
//! Construction is done via the [`TransferFilter::builder()`] method, which
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

/// Placeholder struct representing the data associated with a contract transfer
/// event.
///
/// This struct is primarily used within this module for demonstrating and
/// testing the `TransferFilter::matches` logic. The actual event type provided
/// by the event source (e.g., representing a transfer event parsed from a
/// contract's logs) must be downcastable to a type that exposes both a
/// `contract_id` and a numeric `amount` for the filter to function correctly.
///
/// The fields here represent the minimal information needed *by the filter*.
#[derive(Debug, Clone)]
pub struct TransferEventData {
    /// The ID of the contract that emitted the event (e.g., token contract).
    pub contract_id: String,
    /// The amount transferred.
    pub amount: u64,
    // Other fields like sender, receiver, memo are ignored by this filter.
}

/// A [`Filter`] implementation for contract transfer event subscriptions.
///
/// This filter checks incoming events based on a mandatory `contract_id` and an
/// optional minimum `amount`.
///
/// - The `contract_id` must match the contract that emitted the transfer event.
/// - If `min_amount` is `Some(min)`, the event's `amount` must be greater than
///   or equal to `min`.
/// - If `min_amount` is `None`, any transfer amount from the target contract
///   matches.
///
/// The `include_metadata` field determines the desired verbosity of the
/// resulting notification payload but does not affect whether an event
/// `matches` this filter.
///
/// Use the [`TransferFilter::builder()`] to construct instances. This requires
/// setting the `contract_id`.
///
/// # Examples
///
/// ```rust
/// use rusk::jsonrpc::infrastructure::subscription::filters::{Filter, TransferFilter, TransferEventData};
///
/// // Build a filter for a specific contract, minimum amount 1000, requesting metadata
/// let filter = TransferFilter::builder()
///     .contract_id("token_contract_abc".to_string())
///     .min_amount(Some(1000))
///     .include_metadata(true) // Does not affect matching
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
/// // Check matching logic
/// assert!(filter.matches(&event_match));
/// assert!(filter.matches(&event_match_exact));
/// assert!(!filter.matches(&event_no_match_amount)); // Amount too low
/// assert!(!filter.matches(&event_no_match_contract)); // Wrong contract
/// assert!(!filter.matches(&NonTransferEvent)); // Wrong type
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
    /// Creates a new type-state builder for `TransferFilter`.
    ///
    /// The builder starts in a state requiring the `contract_id` to be set.
    pub fn builder() -> TransferFilterBuilder<NoContractIdT> {
        TransferFilterBuilder::new()
    }

    /// Returns the contract ID that this filter targets (e.g., the token
    /// contract address).
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    /// Returns the optional minimum transfer amount required for an event to
    /// match this filter.
    ///
    /// If `Some(amount)`, only transfers of `amount` or greater will match.
    /// If `None`, the filter does not discriminate based on amount.
    pub fn min_amount(&self) -> Option<u64> {
        self.min_amount
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

impl Filter for TransferFilter {
    /// Checks if a given event matches the criteria of this transfer filter.
    ///
    /// 1. It attempts to downcast the `event` to [`TransferEventData`] (or the
    ///    actual expected transfer event type).
    /// 2. If the downcast succeeds, it checks if the event's `contract_id`
    ///    matches the filter's mandatory `contract_id`. If not, it returns
    ///    `false`.
    /// 3. If the contract IDs match and the filter specifies a `min_amount`
    ///    (`Some(min)`), it checks if the event's `amount` is greater than or
    ///    equal to `min`. If not, it returns `false`.
    /// 4. If the contract IDs match and either the filter does *not* specify a
    ///    `min_amount` (`None`) or the amount condition is met, it returns
    ///    `true`.
    /// 5. If the initial downcast fails, it returns `false`.
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
    ///     .build();
    ///
    /// let event_match = TransferEventData { contract_id: "token_contract_abc".to_string(), amount: 1500 };
    /// let event_no_match_amount = TransferEventData { contract_id: "token_contract_abc".to_string(), amount: 500 };
    /// let event_no_match_contract = TransferEventData { contract_id: "other_contract".to_string(), amount: 2000 };
    /// struct NonTransferEvent;
    ///
    /// assert!(filter.matches(&event_match));
    /// assert!(!filter.matches(&event_no_match_amount));
    /// assert!(!filter.matches(&event_no_match_contract));
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

/// Builder for [`TransferFilter`] using the type-state pattern.
///
/// This ensures the mandatory `contract_id` field is set before the filter can
/// be built. Optional fields like `min_amount` and `include_metadata` can be
/// set at any point before building.
///
/// Start with [`TransferFilter::builder()`].
#[derive(Debug)]
pub struct TransferFilterBuilder<State> {
    state: State,
    min_amount: Option<u64>,
    include_metadata: bool,
    _phantom: PhantomData<State>,
}

// Methods specific to the initial state
impl TransferFilterBuilder<NoContractIdT> {
    /// Creates a new builder instance in the initial state (`NoContractIdT`).
    fn new() -> Self {
        Self {
            state: NoContractIdT,
            min_amount: None,
            include_metadata: false, // Default
            _phantom: PhantomData,
        }
    }
}

// Methods available in any state (setting required/optional fields)
impl<State> TransferFilterBuilder<State> {
    /// Sets the mandatory contract ID for the filter (e.g., the token contract
    /// address).
    ///
    /// This transitions the builder into the [`WithContractIdT`] state,
    /// allowing `build()` to be called.
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

    /// Sets an optional minimum transfer amount for filtering.
    ///
    /// - If `Some(amount)` is provided, the built filter will only match
    ///   transfer events where the transferred amount is greater than or equal
    ///   to `amount`.
    /// - If `None` (the default) is provided, the built filter will match any
    ///   transfer event from the specified contract, regardless of the amount.
    ///
    /// Defaults to `None`.
    ///
    /// # Note on Types
    /// The underlying JSON-RPC subscription parameter for this might be a
    /// string, but the filter internally uses `Option<u64>`. The conversion
    /// from the request parameter (string) to `u64` should happen before
    /// this builder is called.
    pub fn min_amount(mut self, min_amount: Option<u64>) -> Self {
        self.min_amount = min_amount;
        self
    }

    /// Sets whether the subscription requests detailed metadata in transfer
    /// event notifications.
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

// Methods specific to the final state (building the object)
impl TransferFilterBuilder<WithContractIdT> {
    /// Builds the final [`TransferFilter`].
    ///
    /// This method is only available once the mandatory `contract_id` has been
    /// provided via the
    /// [`contract_id`](TransferFilterBuilder::contract_id) method.
    pub fn build(self) -> TransferFilter {
        TransferFilter {
            contract_id: self.state.0,
            min_amount: self.min_amount,
            include_metadata: self.include_metadata,
        }
    }
}
