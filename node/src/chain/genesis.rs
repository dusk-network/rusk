// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::Block;


/// Generates the genesis state for the chain per specified network type
pub(crate) fn generate_state() -> Block {
    // TBD
    Block::default()
}
