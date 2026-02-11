// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_rusk_test::{Result, RuskVmConfig, TestContext};

const BLOCK_GAS_LIMIT: u64 = 24_000_000;
const BLOCKS_NUM: u64 = 10;

#[tokio::test(flavor = "multi_thread")]
pub async fn finalization() -> Result<()> {
    let toml = include_str!("../config/multi_transfer.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let tc = TestContext::instantiate(toml, vm_config).await?;

    let roots = empty_blocks(&tc, BLOCKS_NUM, false);
    tc.revert_to_base_root().expect("revert to work");
    let roots_with_finalize = empty_blocks(&tc, BLOCKS_NUM, true);

    // ensure that roots calculation is not influenced by the finalization
    // strategy
    assert_eq!(roots, roots_with_finalize, "roots mismatch");

    Ok(())
}

fn empty_blocks(
    tc: &TestContext,
    blocks: u64,
    finalize: bool,
) -> Vec<[u8; 32]> {
    let mut roots = vec![];

    let base_root = tc.state_root();
    roots.push(base_root);

    for height in 0..blocks {
        let root = tc.empty_block(height).expect("block to be created");
        if finalize {
            let to_merge = roots.last().expect("to exists");
            tc.rusk()
                .finalize_state(root, vec![*to_merge])
                .expect("finalization to work");
        }
        roots.push(root);
    }

    roots
}
