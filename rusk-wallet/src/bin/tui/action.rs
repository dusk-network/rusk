// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Mutex;

use tokio::sync::mpsc;

use super::app::StakeState;
use crate::frontend::{BalanceView, OperationResult};
use crate::transaction_history::TransactionHistory;

/// Global channel for status callbacks from the wallet library.
///
/// The wallet uses `fn(&str)` function pointers for status updates, which
/// cannot capture state. We bridge that gap with a replaceable global sender so
/// we can re-enter the TUI (e.g. after import) without losing status updates.
static STATUS_TX: Mutex<Option<mpsc::UnboundedSender<AsyncResult>>> =
    Mutex::new(None);

/// Initialize/replace the global status sender. Must be called before any
/// wallet operations that emit status messages.
pub fn init_status_channel(tx: mpsc::UnboundedSender<AsyncResult>) {
    *STATUS_TX.lock().expect("status mutex poisoned") = Some(tx);
}

/// Clear the global status sender (e.g. after leaving the TUI).
pub fn clear_status_channel() {
    *STATUS_TX.lock().expect("status mutex poisoned") = None;
}

/// Status callback function compatible with the wallet library's `fn(&str)`
/// signature. Sends status messages to the TUI via the global channel.
pub fn tui_status(msg: &str) {
    if let Some(tx) = STATUS_TX.lock().expect("status mutex poisoned").as_ref()
    {
        let _ = tx.send(AsyncResult::StatusMessage(msg.to_string()));
    } else {
        // In headless mode (or if the TUI hasn't initialized yet), fall back to
        // standard status logging so callers can always use `tui_status`.
        crate::io::status::headless(msg);
    }
}

/// Results from async wallet operations sent back to the TUI event loop.
#[derive(Debug)]
pub enum AsyncResult {
    /// Balance data for a profile
    BalanceUpdate {
        profile_idx: u8,
        balance: BalanceView,
    },
    /// Stake data for a profile
    StakeUpdate {
        profile_idx: u8,
        stake: StakeState,
        attempted_at_tip: Option<u64>,
    },
    /// Sync status update from background sync
    SyncStatus(String),
    /// Status message from wallet operations
    StatusMessage(String),
    /// Current chain tip height from GraphQL
    ChainTipHeight(u64),
    /// Frontend-facing operation result
    Operation(OperationResult),
    /// Transaction history fetched
    HistoryFetched(Vec<TransactionHistory>),
}
