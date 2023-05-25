use std::default;

use anyhow::Result;
use node_data::ledger::Block;

// The sequencer is used to order incoming blocks and provide them
// in the correct order to the Chain when synchronizing.
#[derive(Default)]
pub(crate) struct Sequencer {
    offset: usize,
    block_pool: Vec<Block>,
}

impl Sequencer {
    pub(crate) fn add(&mut self, blk: Block) {
        // TODO:
    }

    // Returns a block by height
    pub(crate) fn get_from_height(&self, height: u64) -> Result<Block> {
        Ok(Block::default())
    }
}

impl Iterator for Sequencer {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO:
        Some(Block::default())
    }
}
