// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Registry, Tx, DB};
use anyhow::{Context, Result};

use dusk_consensus::commons::Block;
use dusk_consensus::messages::Serializable;
use rocksdb_lib::{
    ColumnFamily, DBAccess, DBCommon, DBWithThreadMode, MultiThreaded,
    OptimisticTransactionDB, OptimisticTransactionOptions, Options,
    ReadOptions, SnapshotWithThreadMode, WriteOptions,
};
use std::{marker::PhantomData, path::Path, sync::Arc};

enum TxType {
    ReadWrite,
    ReadOnly,
}

const CF_LEDGER: &str = "cf_ledger";
const CF_CANDIDATES: &str = "cf_candidates";

pub struct Backend {
    rocksdb: Arc<OptimisticTransactionDB>,
}

impl Backend {
    fn begin_tx(
        &self,
        access_type: TxType,
    ) -> Transaction<'_, OptimisticTransactionDB> {
        // Create a new RocksDB transaction
        let write_options = WriteOptions::default();
        let mut tx_options = OptimisticTransactionOptions::default();

        let inner = self.rocksdb.transaction_opt(&write_options, &tx_options);

        // Borrow column families
        let ledger_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER)
            .expect("ledger column family must exist");

        let candidates_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES)
            .expect("candidates column family must exist");

        let snapshot = self.rocksdb.snapshot();

        Transaction::<'_, OptimisticTransactionDB> {
            inner,
            access_type,
            candidates_cf,
            ledger_cf,
            snapshot,
        }
    }
}

impl DB for Backend {
    type T<'a> = Transaction<'a, OptimisticTransactionDB>;

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
                .expect("should be a valid database"),
            ),
        }
    }

    fn view<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(Self::T<'a>) -> Result<()>,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx(TxType::ReadOnly);

        // Execute all read-only transactions in isolation
        f(tx)?;

        Ok(())
    }

    fn update<F>(&self, execute: F) -> Result<()>
    where
        F: for<'a> FnOnce(&Self::T<'a>) -> Result<()>,
    {
        // Create read-write transaction
        let tx = self.begin_tx(TxType::ReadWrite);

        // If f returns err, no commit will be applied into backend
        // storage
        execute(&tx)?;

        // Apply changes in atomic way
        tx.commit()?;

        Ok(())
    }

    fn close(&mut self) {}
}

pub struct Transaction<'db, DB: DBAccess> {
    inner: rocksdb_lib::Transaction<'db, DB>,
    access_type: TxType,

    candidates_cf: &'db ColumnFamily,
    ledger_cf: &'db ColumnFamily,
    snapshot: SnapshotWithThreadMode<'db, DB>,
}

impl<'db, DB: DBAccess> Tx for Transaction<'db, DB> {
    fn store_block(&self, b: &Block, persisted: bool) -> Result<()> {
        let mut serialized = vec![];
        b.header.write(&mut serialized)?;

        self.inner
            .put_cf(self.ledger_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn delete_block(&self, b: &Block) -> Result<()> {
        let key = b.header.hash;
        self.inner.delete_cf(self.ledger_cf, key)?;

        Ok(())
    }

    fn fetch_block(&self, hash: &[u8]) -> Result<Option<Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.ledger_cf, hash)? {
            let b = Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }

    /// Deletes all items from both CF_LEDGER and CF_CANDIDATES column families
    fn clear_database(&self) -> Result<()> {
        // Create an iterator over the column family CF_LEDGER
        let iter = self
            .inner
            .iterator_cf(self.ledger_cf, rocksdb_lib::IteratorMode::Start);

        // Iterate through the CF_LEDGER column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.ledger_cf, key);
            })
            .collect::<Vec<_>>();

        self.clear_candidates()?;
        Ok(())
    }

    fn store_candidate_block(&self, b: Block) -> Result<()> {
        let mut serialized = vec![];
        b.write(&mut serialized)?;

        self.inner
            .put_cf(self.candidates_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn fetch_candidate_block(&self, hash: &[u8]) -> Result<Option<Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.candidates_cf, hash)? {
            let b = Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }

    /// Deletes all items from CF_CANDIDATES column family
    fn clear_candidates(&self) -> Result<()> {
        // Create an iterator over the column family CF_CANDIDATES
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, rocksdb_lib::IteratorMode::Start);

        // Iterate through the CF_CANDIDATES column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.candidates_cf, key);
            })
            .collect::<Vec<_>>();

        Ok(())
    }

    fn commit(self) -> Result<()> {
        if let Err(e) = self.inner.commit() {
            return Err(anyhow::Error::new(e).context("failed to commit"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusk_consensus::commons::{Certificate, Header, Signature};
    #[test]
    fn test_store_block() {
        let t = TestWrapper {
            path: "_test_store_block",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());

            let b = mock_block(100);
            let hash = b.header.hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn.fetch_block(&hash)?.unwrap() == b);
                Ok(())
            });

            assert!(db
                .update(|txn| {
                    txn.clear_database()?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn.fetch_block(&hash)?.is_none());
                Ok(())
            });
        });
    }

    #[test]
    fn test_read_only() {
        let t = TestWrapper {
            path: "_test_read_only",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let b = mock_block(22);
            assert!(db
                .view(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn.fetch_block(&b.header.hash)?.is_none());
                Ok(())
            });
        });
    }

    #[test]
    fn test_transaction_isolation() {
        let t = TestWrapper {
            path: "_test_transaction_isolation",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let b = mock_block(101);
            let hash = b.header.hash;

            db.view(|txn| {
                // Simulate a concurrent update is committed during read-only
                // transaction
                assert!(db
                    .update(|txn| {
                        txn.store_block(&b, false)?;

                        // No need to support Read-Your-Own-Writes
                        assert!(txn.fetch_block(&hash)?.is_none());
                        Ok(())
                    })
                    .is_ok());

                // Asserts that the read-only/view transaction runs in isolation
                assert!(txn.fetch_block(&hash)?.is_none());
                Ok(())
            });

            // Asserts that update was done
            db.view(|txn| {
                assert!(txn.fetch_block(&hash)?.unwrap() == b);
                Ok(())
            });
        });
    }

    struct TestWrapper {
        path: &'static str,
    }

    impl TestWrapper {
        pub fn run<F>(&self, test_func: F)
        where
            F: FnOnce(&str),
        {
            test_func(self.path);

            // Destroy/deletion of a database can happen only after dropping DB.
            let opts = Options::default();
            rocksdb_lib::DB::destroy(&opts, Path::new(&self.path));
        }
    }

    fn mock_block(height: u64) -> Block {
        Block::new(
            Header {
                version: 0,
                height,
                timestamp: 11112222,
                gas_limit: 123456,
                prev_block_hash: [10; 32],
                seed: Signature::default(),
                generator_bls_pubkey: [12; 96],
                state_hash: [13; 32],
                hash: [0; 32],
                cert: Certificate::default(),
            },
            vec![],
        )
        .expect("should be valid hash")
    }
}
