// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Registry, Tx, DB};
use anyhow::Result;
use rocksdb_lib::{
    ColumnFamily, DBCommon, DBWithThreadMode, MultiThreaded,
    OptimisticTransactionDB, Options, ReadOptions, WriteBatch, WriteOptions,
};
use std::{marker::PhantomData, path::Path, sync::Arc};

enum TxType {
    ReadWrite,
    ReadOnly,
}

const CF_LEDGER: &str = "cf_ledger";
const CF_CANDIDATES: &str = "cf_candidates";

/// Unit test
/// draft PR
pub struct Backend {
    rocksdb: Arc<OptimisticTransactionDB>,
}

impl Backend {
    fn begin_tx(&'static self, access_type: TxType) -> Transaction {
        // Create a new RocksDB transaction
        let txn = self.rocksdb.transaction();

        // Borrow column families
        let ledger_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER)
            .expect("ledger column family must exist");

        let candidates_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER)
            .expect("candidates column family must exist");

        Transaction {
            inner: txn,
            access_type,
            candidates_cf,
            ledger_cf,
        }
    }
}

impl DB for Backend {
    type T = Transaction;

    fn create_or_open(path: String) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_level_compaction_dynamic_level_bytes(true);

        Self {
            rocksdb: Arc::new(
                rocksdb_lib::OptimisticTransactionDB::open_cf(
                    &opts,
                    Path::new(&path),
                    [CF_LEDGER, CF_CANDIDATES],
                )
                .expect("should be a valid path"),
            ),
        }
    }

    fn view<F>(&'static mut self, f: F) -> Result<()>
    where
        F: FnOnce(&Transaction) -> Result<()>,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx(TxType::ReadOnly);

        // Execute all read-only transactions in isolation
        f(&tx)?;

        Ok(())
    }

    fn update<F>(&'static self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Transaction) -> Result<()>,
    {
        // Create read-write transaction
        let mut tx = self.begin_tx(TxType::ReadWrite);

        // If f returns err, no commit will be applied into backend
        // storage
        f(&mut tx)?;

        // Apply changes in atomic way
        tx.commit()?;

        Ok(())
    }

    fn close(&mut self) {}
}

pub struct Transaction {
    inner: rocksdb_lib::Transaction<'static, OptimisticTransactionDB>,
    access_type: TxType,

    candidates_cf: &'static ColumnFamily,
    ledger_cf: &'static ColumnFamily,
}

impl Tx for Transaction {
    // Read-only transactions.
    // fn fetch_block_header(&self, hash: &[u8]) -> Result<&Header> {
    //     // Code for fetching block header here
    //     // ...
    // }

    // fn fetch_block_txs(&self, hash: &[u8]) -> Result<Vec<ContractCall>> {
    //     // Code for fetching block transactions here
    //     // ...
    // }

    // fn fetch_block_tx_by_hash(&self, tx_id: &[u8]) -> Result<(ContractCall,
    // u32, &[u8])> {     // Code for fetching block transaction by hash
    // here     // ...
    // }

    fn fetch_block_hash_by_height(&self, height: u64) -> Result<&[u8]> {
        Err(anyhow::Error::msg("message"))
    }

    fn fetch_block_exists(&self, hash: &[u8]) -> Result<bool> {
        anyhow::Ok(false)
    }

    // fn fetch_block_by_state_root(&self, from_height: u64, state_root: &[u8])
    // -> Result<&Block> {     // Code for fetching block by state root here
    //     // ...
    // }

    fn fetch_registry(&self) -> Result<Registry> {
        anyhow::Ok(Registry::default())
    }

    fn fetch_current_height(&self) -> Result<u64> {
        anyhow::Ok(0)
    }

    fn fetch_block_height_since(
        &self,
        since_unix_time: i64,
        offset: u64,
    ) -> Result<u64> {
        anyhow::Ok(0)
    }

    /// Deletes all items from CF_CANDIDATES column family
    fn clear_candidate_messages(&mut self) -> Result<()> {
        // Create an iterator over the column family CF_CANDIDATES
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, rocksdb_lib::IteratorMode::Start);

        // Iterate through the CF_CANDIDATES column family and delete all items
        iter.map(Result::unwrap).map(|(key, _)| {
            self.inner.delete_cf(self.candidates_cf, key);
        });

        anyhow::Ok(())
    }

    /// Deletes all items from CF_LEDGER column family
    fn clear_database(&mut self) -> Result<()> {
        // Create an iterator over the column family CF_CANDIDATES
        let iter = self
            .inner
            .iterator_cf(self.ledger_cf, rocksdb_lib::IteratorMode::Start);

        // Iterate through the CF_CANDIDATES column family and delete all items
        iter.map(Result::unwrap).map(|(key, _)| {
            self.inner.delete_cf(self.candidates_cf, key);
        });

        anyhow::Ok(())
    }

    fn commit(self) -> Result<()> {
        self.inner.commit()?;
        anyhow::Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        anyhow::Ok(())
    }

    fn close(&mut self) {}

    // Read-write transactions
    // fn store_block(&mut self, block: &Block, persisted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;

    #[test]
    fn test_clear_candidates() {
        lazy_static! {
            static ref TEST_DB: Backend =
                Backend::create_or_open("/tmp/db_test/".to_owned());
        }

        let res = TEST_DB.update(|txn| {
            txn.clear_candidate_messages()?;
            txn.clear_database()?;

            anyhow::Ok(())
        });

        assert!(res.is_ok());
    }
}
