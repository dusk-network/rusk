// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod rocksdb;
use anyhow::Result;
pub trait DB: Send + Sync + 'static {
    type T: Tx;

    /// Creates or open a database located at this path.
    ///
    /// Panics if openning db or creating one fails.
    fn create_or_open(path: String) -> Self;

    /// Provides a managed execution of a read-only isolated transaction.
    fn view<F>(&'static mut self, f: F) -> Result<()>
    where
        F: FnOnce(&Self::T) -> Result<()>;

    /// Provides a managed execution of a read-write atomic transaction.
    ///
    /// An atomic transaction is an indivisible and irreducible series of
    /// database operations such that either all occur, or nothing occurs.
    ///
    /// Transaction commit will happen only if no error is returned by `fn`
    /// and no panic is raised on `fn` execution.
    fn update<F>(&'static self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Self::T) -> Result<()>;

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.
///
/// TODO: Uncomment APIs when ContractCall and Block, Header and certificate are
/// defined
pub trait Tx {
    // Read-only transactions.
    //fn fetch_block_header(&self, hash: &[u8]) -> Result<&Header>;
    //fn fetch_block_txs(&self, hash: &[u8]) -> Result<Vec<ContractCall>>;
    //fn fetch_block_tx_by_hash(
    //    &self,
    //    tx_id: &[u8],
    //) -> Result<(ContractCall, u32, &[u8])>;

    fn fetch_block_hash_by_height(&self, height: u64) -> Result<&[u8]>;
    fn fetch_block_exists(&self, hash: &[u8]) -> Result<bool>;
    // fn fetch_block_by_state_root(
    //    &self,
    //    from_height: u64,
    //    state_root: &[u8],
    //) -> Result<&Block>;
    fn fetch_registry(&self) -> Result<Registry>;

    // Read-write transactions
    //fn store_block(&mut self, block: &Block, persisted: bool) -> Result<()>;
    //fn delete_block(&mut self, b: &Block) -> Result<()>;
    //fn fetch_block(&self, hash: &[u8]) -> Result<&Block>;
    fn fetch_current_height(&self) -> Result<u64>;
    fn fetch_block_height_since(
        &self,
        since_unix_time: i64,
        offset: u64,
    ) -> Result<u64>;
    //fn store_candidate_message(&mut self, cm: Block) -> Result<()>;
    //fn fetch_candidate_message(&self, hash: &[u8]) -> Result<Block>;
    fn clear_candidate_messages(&mut self) -> Result<()>;
    fn clear_database(&mut self) -> Result<()>;
    fn commit(self) -> Result<()>;
    fn rollback(&mut self) -> Result<()>;
    fn close(&mut self);
}

#[derive(Default)]
pub struct Registry {
    pub tip_hash: [u8; 32],
    pub persisted_hash: [u8; 32],
}
