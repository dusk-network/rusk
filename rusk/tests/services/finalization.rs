// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use rusk::{node::RuskVmConfig, Result, Rusk};
use tempfile::tempdir;

use crate::common::state::{generator_procedure2, new_state};

const BLOCK_GAS_LIMIT: u64 = 24_000_000;
const BLOCKS_NUM: u64 = 10;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot =
        toml::from_str(include_str!("../config/multi_transfer.toml"))
            .expect("Cannot deserialize config");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);

    new_state(dir, &snapshot, vm_config)
}

#[tokio::test(flavor = "multi_thread")]
pub async fn finalization() -> Result<()> {
    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let roots = empty_blocks(&rusk, BLOCKS_NUM, false);
    rusk.revert_to_base_root().expect("revert to work");
    let roots_with_finalize = empty_blocks(&rusk, BLOCKS_NUM, true);

    // ensure that roots calculation is not influenced by the finalization
    // strategy
    assert_eq!(roots, roots_with_finalize, "roots mismatch");

    Ok(())
}

fn empty_blocks(rusk: &Rusk, blocks: u64, finalize: bool) -> Vec<[u8; 32]> {
    let mut roots = vec![];

    let base_root = rusk.state_root();
    roots.push(base_root);

    for height in 0..blocks {
        let (_, root) = generator_procedure2(
            &rusk,
            &[],
            height,
            BLOCK_GAS_LIMIT,
            vec![],
            None,
            None,
        )
        .expect("block to be created");
        if finalize {
            let to_merge = roots.last().expect("to exists");
            rusk.finalize_state(root, vec![*to_merge])
                .expect("finalization to work");
        }
        roots.push(root);
    }

    roots
}
