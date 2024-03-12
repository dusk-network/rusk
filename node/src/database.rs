// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::path::Path;

pub mod rocksdb;

use anyhow::Result;
use node_data::ledger;
use node_data::ledger::{Label, SpentTransaction};

pub trait DB: Send + Sync + 'static {
    type P<'a>: Persist;

    /// Creates or open a database located at this path.
    ///
    /// Panics if opening db or creating one fails.
    fn create_or_open<T>(path: T) -> Self
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

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.

pub trait Ledger {
    // Read-write transactions
    fn store_block(
        &self,
        header: &ledger::Header,
        txs: &[SpentTransaction],
        label: Label,
    ) -> Result<()>;

    fn delete_block(&self, b: &ledger::Block) -> Result<()>;
    fn fetch_block_header(
        &self,
        hash: &[u8],
    ) -> Result<Option<(ledger::Header, Vec<[u8; 32]>)>>;

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
        tx_hash: &[u8],
    ) -> Result<Option<ledger::SpentTransaction>>;

    fn get_ledger_tx_exists(&self, tx_hash: &[u8]) -> Result<bool>;

    fn fetch_block_label_by_height(&self, height: u64)
        -> Result<Option<Label>>;
}

pub trait Candidate {
    // Read-write transactions
    fn store_candidate_block(&self, cm: ledger::Block) -> Result<()>;
    fn fetch_candidate_block(
        &self,
        hash: &[u8],
    ) -> Result<Option<ledger::Block>>;
    fn clear_candidates(&self) -> Result<()>;

    fn delete<F>(&self, closure: F) -> Result<()>
    where
        F: FnOnce(u64) -> bool + std::marker::Copy;

    fn count(&self) -> usize;
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

    /// Get transactions hash from the mempool, searching by nullifiers
    fn get_txs_by_nullifiers(&self, n: &[[u8; 32]]) -> HashSet<[u8; 32]>;

    /// Get an iterator over the mempool transactions sorted by gas price
    fn get_txs_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = ledger::Transaction> + '_>>;

    /// Get an iterator over the mempool transactions hash by gas price
    fn get_txs_hashes_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>>;

    /// Get all transactions hashes.
    fn get_txs_hashes(&self) -> Result<Vec<[u8; 32]>>;
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
}

pub fn into_array<const N: usize>(value: &[u8]) -> [u8; N] {
    let mut res = [0u8; N];
    res.copy_from_slice(&value[0..N]);
    res
}
