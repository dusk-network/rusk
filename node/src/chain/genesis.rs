// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::{Block, Header};

/// Generates the genesis state for the chain per specified network type
pub(crate) fn generate_state(state_hash: [u8; 32]) -> Block {
    Block::new(
        Header {
            // Mon Mar 25 2024 11:00:00 GMT+0000
            timestamp: 1711364400,
            state_hash,
            ..Default::default()
        },
        vec![],
    )
    .expect("block should be valid")
}
