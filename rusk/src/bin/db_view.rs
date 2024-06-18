// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::rocksdb::{Backend, MD_HASH_KEY};
use node::database::{DBViewer, Ledger, Metadata, DB};

pub struct DBView {
    db: Backend,
}

impl DBView {
    pub fn new(db: Backend) -> Self {
        Self { db }
    }
}

impl DBViewer for DBView {
    fn fetch_block_hash(
        &self,
        block_height: u64,
    ) -> Result<Option<[u8; 32]>, anyhow::Error> {
        self.db.view(|t| t.fetch_block_hash_by_height(block_height))
    }
    fn fetch_tip_height(&self) -> anyhow::Result<u64, anyhow::Error> {
        let tip_block = self.db.view(|t| {
            t.op_read(MD_HASH_KEY)
                .ok()?
                .and_then(|hash| t.fetch_block(&hash[..]).ok()?)
        });
        match tip_block {
            Some(block) => Ok(block.header().height),
            _ => Err(anyhow::anyhow!("tip block not found")),
        }
    }
}
