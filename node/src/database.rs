// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::path::Path;

pub mod rocksdb;

use anyhow::Result;

use node_data::ledger::{self, Fault, Label, SpendingId, SpentTransaction};
use node_data::message::ConsensusHeader;

use serde::{Deserialize, Serialize};

pub struct LightBlock {
    pub header: ledger::Header,
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
        F: for<'a> FnOnce(Self::P<'a>) -> T;

    /// Provides a managed execution of a read-write atomic transaction.
    ///
    /// An atomic transaction is an indivisible and irreducible series of
    /// database operations such that either all occur, or nothing occurs.
    ///
    /// Transaction commit will happen only if no error is returned by `fn`
    /// and no panic is raised on `fn` execution.
    fn update<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<T>;

    fn update_dry_run<F, T>(&self, dry_run: bool, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<T>;

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.

pub trait Ledger {
    /// Read-write transactions
    /// Returns disk footprint of the committed transaction
    fn store_block(
        &self,
        header: &ledger::Header,
        txs: &[SpentTransaction],
        faults: &[Fault],
        label: Label,
    ) -> Result<usize>;

    fn delete_block(&self, b: &ledger::Block) -> Result<()>;
    fn fetch_block_header(&self, hash: &[u8])
        -> Result<Option<ledger::Header>>;

    fn fetch_light_block(&self, hash: &[u8]) -> Result<Option<LightBlock>>;

    fn fetch_block(&self, hash: &[u8]) -> Result<Option<ledger::Block>>;
    fn fetch_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<[u8; 32]>>;
    fn fetch_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<ledger::Block>>;

    fn get_block_exists(&self, hash: &[u8]) -> Result<bool>;

    fn get_ledger_tx_by_hash(
        &self,
        tx_id: &[u8],
    ) -> Result<Option<ledger::SpentTransaction>>;

    fn get_ledger_tx_exists(&self, tx_id: &[u8]) -> Result<bool>;

    fn fetch_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<([u8; 32], Label)>>;

    fn store_block_label(
        &self,
        height: u64,
        hash: &[u8; 32],
        label: Label,
    ) -> Result<()>;

    fn fetch_faults_by_block(&self, start_height: u64) -> Result<Vec<Fault>>;
    fn fetch_faults(&self, faults_ids: &[[u8; 32]]) -> Result<Vec<Fault>>;
}

pub trait Candidate {
    // Read-write transactions
    fn store_candidate_block(&self, cm: ledger::Block) -> Result<()>;
    fn fetch_candidate_block(
        &self,
        hash: &[u8],
    ) -> Result<Option<ledger::Block>>;

    /// Fetches a candidate block by lookup key (prev_block_hash, iteration).
    fn fetch_candidate_block_by_iteration(
        &self,
        ch: &ConsensusHeader,
    ) -> Result<Option<ledger::Block>>;

    fn clear_candidates(&self) -> Result<()>;

    fn delete<F>(&self, closure: F) -> Result<()>
    where
        F: FnOnce(u64) -> bool + std::marker::Copy;

    fn count(&self) -> usize;
}

pub trait Mempool {
    /// Adds a transaction to the mempool with a timestamp.
    fn add_tx(&self, tx: &ledger::Transaction, timestamp: u64) -> Result<()>;

    /// Gets a transaction from the mempool.
    fn get_tx(&self, tx_id: [u8; 32]) -> Result<Option<ledger::Transaction>>;

    /// Checks if a transaction exists in the mempool.
    fn get_tx_exists(&self, tx_id: [u8; 32]) -> Result<bool>;

    /// Deletes a transaction from the mempool.
    ///
    /// If `cascade` is true, all dependant transactions are deleted
    ///
    /// Return a vector with all the deleted tx_id
    fn delete_tx(
        &self,
        tx_id: [u8; 32],
        cascade: bool,
    ) -> Result<Vec<[u8; 32]>>;

    /// Get transactions hash from the mempool, searching by spendable ids
    fn get_txs_by_spendable_ids(&self, n: &[SpendingId]) -> HashSet<[u8; 32]>;

    /// Get an iterator over the mempool transactions sorted by gas price
    fn get_txs_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = ledger::Transaction> + '_>>;

    /// Get an iterator over the mempool transactions hash by gas price
    fn get_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>>;

    /// Get an iterator over the mempool transactions hash by gas price (asc)
    fn get_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>>;

    /// Get all transactions hashes.
    fn get_txs_ids(&self) -> Result<Vec<[u8; 32]>>;

    /// Get all expired transactions.
    fn get_expired_txs(&self, timestamp: u64) -> Result<Vec<[u8; 32]>>;

    /// Number of persisted transactions
    fn txs_count(&self) -> usize;
}

pub trait Metadata {
    /// Assigns an value to a key in the Metadata CF
    fn op_write<T: AsRef<[u8]>>(&self, key: &[u8], value: T) -> Result<()>;

    /// Reads an value of a key from the Metadata CF
    fn op_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
}

pub trait Persist:
    Ledger + Candidate + Mempool + Metadata + core::fmt::Debug
{
    // Candidate block functions

    fn clear_database(&self) -> Result<()>;
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
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            blocks_cf_max_write_buffer_size: 1024 * 1024, // 1 MiB
            mempool_cf_max_write_buffer_size: 10 * 1024 * 1024, // 10 MiB
            blocks_cf_disable_block_cache: true,
            enable_debug: false,
        }
    }
}
