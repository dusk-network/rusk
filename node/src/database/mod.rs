// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

pub mod rocksdb;

use anyhow::Result;
use node_data::ledger;

pub trait DB: Send + Sync + 'static {
    type P<'a>: Persist;

    /// Creates or open a database located at this path.
    ///
    /// Panics if opening db or creating one fails.
    fn create_or_open<T>(path: T) -> Self
    where
        T: AsRef<Path>;

    /// Provides a managed execution of a read-only isolated transaction.
    fn view<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(Self::P<'a>) -> Result<()>;

    /// Provides a managed execution of a read-write atomic transaction.
    ///
    /// An atomic transaction is an indivisible and irreducible series of
    /// database operations such that either all occur, or nothing occurs.
    ///
    /// Transaction commit will happen only if no error is returned by `fn`
    /// and no panic is raised on `fn` execution.
    fn update<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<()>;

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.

pub trait Ledger {
    // Read-write transactions
    fn store_block(&self, b: &ledger::Block, persisted: bool) -> Result<()>;
    fn delete_block(&self, b: &ledger::Block) -> Result<()>;
    fn fetch_block(&self, hash: &[u8]) -> Result<Option<ledger::Block>>;
    fn fetch_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<[u8; 32]>>;

    fn get_block_exists(&self, hash: &[u8]) -> Result<bool>;

    fn get_ledger_tx_by_hash(
        &self,
        tx_hash: &[u8],
    ) -> Result<Option<ledger::Transaction>>;

    fn get_ledger_tx_exists(&self, tx_hash: &[u8]) -> Result<bool>;
}

pub trait Candidate {
    // Read-write transactions
    fn store_candidate_block(&self, cm: ledger::Block) -> Result<()>;
    fn fetch_candidate_block(
        &self,
        hash: &[u8],
    ) -> Result<Option<ledger::Block>>;
    fn clear_candidates(&self) -> Result<()>;
}

pub trait Mempool {
    /// Adds a transaction to the mempool.
    fn add_tx(&self, tx: &ledger::Transaction) -> Result<()>;

    /// Gets a transaction from the mempool.
    fn get_tx(&self, tx_hash: [u8; 32]) -> Result<Option<ledger::Transaction>>;

    /// Checks if a transaction exists in the mempool.
    fn get_tx_exists(&self, tx_hash: [u8; 32]) -> Result<bool>;

    /// Deletes a transaction from the mempool.
    fn delete_tx(&self, tx_hash: [u8; 32]) -> Result<bool>;

    /// Checks if any of the passed nullifiers exists in the mempool.
    fn get_any_nullifier_exists(&self, nullifiers: Vec<[u8; 32]>) -> bool;

    /// Get all or subset of mempool transactions sorted by fee limited by
    /// max_gas_limit.
    fn get_txs_sorted_by_fee(
        &self,
        max_gas_limit: u64,
    ) -> Result<Vec<ledger::Transaction>>;

    /// Get all transactions hashes.
    fn get_txs_hashes(&self) -> Result<Vec<[u8; 32]>>;
}

pub trait Persist: Ledger + Candidate + Mempool + core::fmt::Debug {
    // Candidate block functions

    fn clear_database(&self) -> Result<()>;
    fn commit(self) -> Result<()>;
}

#[derive(Default)]
pub struct Registry {
    pub tip_hash: [u8; 32],
    pub persisted_hash: [u8; 32],
}
