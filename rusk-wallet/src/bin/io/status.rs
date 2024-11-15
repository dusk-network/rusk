// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use tracing::info;

/// Prints an interactive status message
pub(crate) fn interactive(_status: &str) {
    // FIXME: We currently don't print callback
    // messages from wallet functions because we
    // haven't found a constructive way to do so
    // See issue #2962
}

/// Logs the status message at info level
pub(crate) fn headless(status: &str) {
    info!(status);
}
