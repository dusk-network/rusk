// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::path::Path;

pub mod rocksdb;

use anyhow::Result;
use node_data::ledger::{
    Block, BlockWithSpentTransactions, Fault, Header, Label, SpendingId,
    SpentTransaction, Transaction,
};
use node_data::message::{payload, ConsensusHeader};
use serde::{Deserialize, Serialize};

pub struct LightBlock {
    pub header: Header,
    pub transactions_ids: Vec<[u8; 32]>,
    pub faults_ids: Vec<[u8; 32]>,
}

pub trait DB: Send + Sync + 'static {
    type P<'a>: Persist;

    /// Creates or open a database located at this path.
    ///
    /// Panics if opening db or creating one fails.
    fn create_or_open<T>(path: T, opts: DatabaseOptions) -> Self
    where
        T: AsRef<Path>;

    /// Provides a managed execution of a read-only isolated transaction.
    fn view<F, T>(&self, f: F) -> T
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> T;

    /// Provides a managed execution of a read-write atomic transaction.
    ///
    /// An atomic transaction is an indivisible and irreducible series of
    /// database operations such that either all occur, or nothing occurs.
    ///
    /// Transaction commit will happen only if no error is returned by `fn`
    /// and no panic is raised on `fn` execution.
    fn update<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(&mut Self::P<'a>) -> Result<T>;

    fn update_dry_run<F, T>(&self, dry_run: bool, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(&mut Self::P<'a>) -> Result<T>;

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.

pub trait Ledger {
    /// Read-write transactions
    /// Returns disk footprint of the committed transaction
    fn store_block(
        &mut self,
        header: &Header,
        txs: &[SpentTransaction],
        faults: &[Fault],
        label: Label,
    ) -> Result<usize>;

    fn delete_block(&mut self, b: &Block) -> Result<()>;
    fn block_header(&self, hash: &[u8]) -> Result<Option<Header>>;

    fn light_block(&self, hash: &[u8]) -> Result<Option<LightBlock>>;

    fn block(&self, hash: &[u8]) -> Result<Option<Block>>;
    fn block_with_spent_transactions(
        &self,
        hash: &[u8],
    ) -> Result<Option<BlockWithSpentTransactions>>;
    fn block_hash_by_height(&self, height: u64) -> Result<Option<[u8; 32]>>;
    fn block_by_height(&self, height: u64) -> Result<Option<Block>>;

    fn block_exists(&self, hash: &[u8]) -> Result<bool>;

    fn ledger_tx(&self, tx_id: &[u8]) -> Result<Option<SpentTransaction>>;
    fn ledger_txs(
        &self,
        tx_ids: Vec<&[u8; 32]>,
    ) -> Result<Vec<SpentTransaction>>;

    fn ledger_tx_exists(&self, tx_id: &[u8]) -> Result<bool>;

    fn block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<([u8; 32], Label)>>;

    fn store_block_label(
        &mut self,
        height: u64,
        hash: &[u8; 32],
        label: Label,
    ) -> Result<()>;

    fn faults_by_block(&self, start_height: u64) -> Result<Vec<Fault>>;
    fn faults(&self, faults_ids: &[[u8; 32]]) -> Result<Vec<Fault>>;
}

pub trait ConsensusStorage {
    /// Candidate Storage
    fn store_candidate(&mut self, cm: Block) -> Result<()>;
    fn candidate(&self, hash: &[u8]) -> Result<Option<Block>>;

    /// Fetches a candidate block by lookup key (prev_block_hash, iteration).
    fn candidate_by_iteration(
        &self,
        ch: &ConsensusHeader,
    ) -> Result<Option<Block>>;

    fn clear_candidates(&mut self) -> Result<()>;

    fn delete_candidate<F>(&mut self, closure: F) -> Result<()>
    where
        F: FnOnce(u64) -> bool + std::marker::Copy;

    fn count_candidates(&self) -> usize;

    /// ValidationResult Storage
    fn store_validation_result(
        &mut self,
        ch: &ConsensusHeader,
        vr: &payload::ValidationResult,
    ) -> Result<()>;

    fn validation_result(
        &self,
        ch: &ConsensusHeader,
    ) -> Result<Option<payload::ValidationResult>>;

    fn clear_validation_results(&mut self) -> Result<()>;

    fn delete_validation_results<F>(&mut self, closure: F) -> Result<()>
    where
        F: FnOnce([u8; 32]) -> bool + std::marker::Copy;

    fn count_validation_results(&self) -> usize;
}

pub trait Mempool {
    /// Adds a transaction to the mempool with a timestamp.
    fn store_mempool_tx(
        &mut self,
        tx: &Transaction,
        timestamp: u64,
    ) -> Result<()>;

    /// Gets a transaction from the mempool.
    fn mempool_tx(&self, tx_id: [u8; 32]) -> Result<Option<Transaction>>;

    /// Checks if a transaction exists in the mempool.
    fn mempool_tx_exists(&self, tx_id: [u8; 32]) -> Result<bool>;

    /// Deletes a transaction from the mempool.
    ///
    /// If `cascade` is true, all dependant transactions are deleted
    ///
    /// Return a vector with all the deleted tx_id
    fn delete_mempool_tx(
        &mut self,
        tx_id: [u8; 32],
        cascade: bool,
    ) -> Result<Vec<[u8; 32]>>;

    /// Get transactions hash from the mempool, searching by spendable ids
    fn mempool_txs_by_spendable_ids(
        &self,
        n: &[SpendingId],
    ) -> HashSet<[u8; 32]>;

    /// Get an iterator over the mempool transactions sorted by gas price
    fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Box<dyn Iterator<Item = Transaction> + '_>;

    /// Get an iterator over the mempool transactions hash by gas price
    fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>;

    /// Get an iterator over the mempool transactions hash by gas price (asc)
    fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>;

    /// Get all transactions hashes.
    fn mempool_txs_ids(&self) -> Result<Vec<[u8; 32]>>;

    /// Get all expired transactions.
    fn mempool_expired_txs(&self, timestamp: u64) -> Result<Vec<[u8; 32]>>;

    /// Number of persisted transactions
    fn mempool_txs_count(&self) -> usize;
}

pub trait Metadata {
    /// Assigns an value to a key in the Metadata CF
    fn op_write<T: AsRef<[u8]>>(&mut self, key: &[u8], value: T) -> Result<()>;

    /// Reads an value of a key from the Metadata CF
    fn op_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
}

pub trait Persist:
    Ledger + ConsensusStorage + Mempool + Metadata + core::fmt::Debug
{
    // Candidate block functions

    fn clear_database(&mut self) -> Result<()>;
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

pub fn into_array<const N: usize>(value: &[u8]) -> [u8; N] {
    let mut res = [0u8; N];
    res.copy_from_slice(&value[0..N]);
    res
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DatabaseOptions {
    /// Max write buffer size per Blocks-related CF. By default, there are two
    /// write buffers (MemTables) per CF.
    pub blocks_cf_max_write_buffer_size: usize,

    /// Disables Block Cache for non-Mempool CFs
    ///
    /// Block Cache is useful in optimizing DB reads. For
    /// non-block-explorer nodes, DB reads for block retrieval should
    /// not be buffered in memory.
    pub blocks_cf_disable_block_cache: bool,

    /// Max write buffer size per Mempool CF.
    pub mempool_cf_max_write_buffer_size: usize,

    /// Enables a set of flags for collecting DB stats as log data.
    pub enable_debug: bool,

    /// Create the database if missing
    pub create_if_missing: bool,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            blocks_cf_max_write_buffer_size: 1024 * 1024, // 1 MiB
            mempool_cf_max_write_buffer_size: 10 * 1024 * 1024, // 10 MiB
            blocks_cf_disable_block_cache: true,
            enable_debug: false,
            create_if_missing: true,
        }
    }
}
