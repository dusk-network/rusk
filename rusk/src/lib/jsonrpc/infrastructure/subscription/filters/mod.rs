// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Provides the event filtering mechanism for WebSocket subscriptions.
//!
//! This module defines the core [`Filter`] trait and specific filter
//! implementations used by the
//! [`SubscriptionManager`](crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager)
//! to determine which events should be sent to which subscribers.
//!
//! # Overview
//!
//! When a client subscribes to a WebSocket topic (e.g., new blocks, contract
//! events), they can often provide parameters to narrow down the events they
//! are interested in. These parameters are used to construct a specific
//! [`Filter`] implementation.
//!
//! The [`SubscriptionManager`](crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager)
//! stores these filters alongside the subscriber's communication sink. When a
//! new event occurs in the system, the manager iterates through relevant
//! subscriptions, calls the [`Filter::matches`] method for each, and forwards
//! the event only if the method returns `true`.
//!
//! # Core Components
//!
//! * [`Filter`]: The central trait defining the `matches` method. Relies on
//!   `dyn Any` and downcasting for type safety across different event types.
//! * [`BlockFilter`]: Filter associated with block-related events (e.g.,
//!   `subscribeBlockAcceptance`). Primarily checks event type and carries
//!   payload detail flags.
//! * [`ContractFilter`]: Filter associated with generic contract events (e.g.,
//!   `subscribeContractEvents`). Matches based on `contract_id` and optional
//!   `event_names`.
//! * [`TransferFilter`]: Filter associated with contract transfer events (e.g.,
//!   `subscribeContractTransferEvents`). Matches based on `contract_id` and
//!   optional `min_amount`.
//! * [`MempoolFilter`]: Filter associated with mempool events (e.g.,
//!   `subscribeMempoolAcceptance`). Matches based on optional `contract_id`.
//!
//! # Placeholders
//!
//! Note that each filter submodule currently defines a placeholder `*EventData`
//! struct (e.g., [`BlockEventData`], [`ContractEventData`]). These are used for
//! testing and demonstrating the downcasting logic within the `matches` method.
//! The *actual* event types passed to `matches` will depend on the event source
//! and how the `SubscriptionManager` handles them.

mod block_filter;
mod contract_filter;
mod filter;
mod mempool_filter;
mod transfer_filter;

pub use block_filter::*;
pub use contract_filter::*;
pub use filter::*;
pub use mempool_filter::*;
pub use transfer_filter::*;
