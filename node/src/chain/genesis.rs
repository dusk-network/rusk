// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::Block;

pub const DUSK: u64 = 100_000_000;

/// Generates the genesis state for the chain per specified network type.
///
/// NB. For now it should return a hard-coded list of eligible provisioners.
pub(crate) fn generate_state() -> (Block, Provisioners) {
    let mut block = Block::default();

    // Load provisioners keys from external consensus keys files.
    let keys = node_data::bls::load_provisioners_keys(4);
    let mut provisioners = Provisioners::new();

    for (_, (_, pk)) in keys.iter().enumerate() {
        tracing::info!("Adding provisioner: {:#?}", pk);
        provisioners.add_member_with_value(pk.clone(), 1000 * DUSK * 10);
    }

    (block, provisioners)
}
