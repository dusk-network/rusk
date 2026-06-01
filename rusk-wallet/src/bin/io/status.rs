// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_wallet::{WalletStatus, WalletSyncStatus};
use tracing::{error, info, warn};

/// Logs a typed wallet status update (headless mode).
pub(crate) fn wallet_headless(status: WalletStatus) {
    match &status {
        WalletStatus::Warning(_) => warn!("{status}"),
        WalletStatus::Sync(WalletSyncStatus::Error(_)) => error!("{status}"),
        _ => info!("{status}"),
    }
}
