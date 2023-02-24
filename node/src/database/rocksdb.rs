// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Candidate, Ledger, Persist, Registry, DB};
use anyhow::{Context, Result};

use node_data::encoding::*;
use node_data::ledger;
use node_data::Serializable;

use crate::database::Mempool;

use dusk_bytes::Serializable as DuskBytesSerializable;

use rocksdb_lib::{
    ColumnFamily, ColumnFamilyDescriptor, DBAccess, DBCommon, DBWithThreadMode,
    IteratorMode, MultiThreaded, OptimisticTransactionDB,
    OptimisticTransactionOptions, Options, ReadOptions, SnapshotWithThreadMode,
    WriteOptions,
};

use std::io::Read;
use std::{marker::PhantomData, path::Path, sync::Arc};
use tokio::io::AsyncWriteExt;

enum TxType {
    ReadWrite,
    ReadOnly,
}

const CF_LEDGER: &str = "cf_ledger";
const CF_CANDIDATES: &str = "cf_candidates";
const CF_MEMPOOL: &str = "cf_mempool";
const CF_MEMPOOL_NULLIFIERS: &str = "cf_mempool_nullifiers";
const CF_MEMPOOL_FEES: &str = "cf_mempool_fees";

const MAX_MEMPOOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub struct Backend {
    rocksdb: Arc<OptimisticTransactionDB>,
}

impl Backend {
    fn begin_tx(
        &self,
        access_type: TxType,
    ) -> DBTransaction<'_, OptimisticTransactionDB> {
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

        let mempool_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL)
            .expect("mempool column family must exist");

        let nullifiers_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_NULLIFIERS)
            .expect("CF_MEMPOOL_NULLIFIERS column family must exist");

        let fees_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_FEES)
            .expect("CF_MEMPOOL_FEES column family must exist");

        let snapshot = self.rocksdb.snapshot();

        DBTransaction::<'_, OptimisticTransactionDB> {
            inner,
            access_type,
            candidates_cf,
            ledger_cf,
            mempool_cf,
            nullifiers_cf,
            fees_cf,
            snapshot,
        }
    }
}

impl DB for Backend {
    type P<'a> = DBTransaction<'a, OptimisticTransactionDB>;

    fn create_or_open(path: String) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_level_compaction_dynamic_level_bytes(true);

        // Configure CF_MEMPOOL column family so it benefits from low
        // write-latency of L0
        let mut mp_opts = Options::default();
        mp_opts.set_write_buffer_size(MAX_MEMPOOL_SIZE);

        // Disable WAL by default
        mp_opts.set_manual_wal_flush(true);

        // Disable flush-to-disk by default
        mp_opts.set_disable_auto_compactions(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_LEDGER, Options::default()),
            ColumnFamilyDescriptor::new(CF_CANDIDATES, Options::default()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_NULLIFIERS, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_FEES, mp_opts),
        ];

        Self {
            rocksdb: Arc::new(
                rocksdb_lib::OptimisticTransactionDB::open_cf_descriptors(
                    &opts,
                    Path::new(&path),
                    cfs,
                )
                .expect("should be a valid database"),
            ),
        }
    }

    fn view<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(Self::P<'a>) -> Result<()>,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx(TxType::ReadOnly);

        // Execute all read-only transactions in isolation
        f(tx)?;

        Ok(())
    }

    fn update<F>(&self, execute: F) -> Result<()>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<()>,
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

pub struct DBTransaction<'db, DB: DBAccess> {
    inner: rocksdb_lib::Transaction<'db, DB>,
    access_type: TxType,

    candidates_cf: &'db ColumnFamily,
    ledger_cf: &'db ColumnFamily,
    mempool_cf: &'db ColumnFamily,
    nullifiers_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    snapshot: SnapshotWithThreadMode<'db, DB>,
}

impl<'db, DB: DBAccess> Ledger for DBTransaction<'db, DB> {
    fn store_block(&self, b: &ledger::Block, persisted: bool) -> Result<()> {
        let mut serialized = vec![];
        b.header.write(&mut serialized)?;

        self.inner
            .put_cf(self.ledger_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn delete_block(&self, b: &ledger::Block) -> Result<()> {
        let key = b.header.hash;
        self.inner.delete_cf(self.ledger_cf, key)?;

        Ok(())
    }

    fn fetch_block(&self, hash: &[u8]) -> Result<Option<ledger::Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.ledger_cf, hash)? {
            let b = ledger::Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }
}

impl<'db, DB: DBAccess> Candidate for DBTransaction<'db, DB> {
    fn store_candidate_block(&self, b: ledger::Block) -> Result<()> {
        let mut serialized = vec![];
        b.write(&mut serialized)?;

        self.inner
            .put_cf(self.candidates_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn fetch_candidate_block(
        &self,
        hash: &[u8],
    ) -> Result<Option<ledger::Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.candidates_cf, hash)? {
            let b = ledger::Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }

    /// Deletes all items from CF_CANDIDATES column family
    fn clear_candidates(&self) -> Result<()> {
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, IteratorMode::Start);

        // Iterate through the CF_CANDIDATES column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.candidates_cf, key);
            })
            .collect::<Vec<_>>();

        Ok(())
    }
}

impl<'db, DB: DBAccess> Persist for DBTransaction<'db, DB> {
    /// Deletes all items from both CF_LEDGER and CF_CANDIDATES column families
    fn clear_database(&self) -> Result<()> {
        // Create an iterator over the column family CF_LEDGER
        let iter = self.inner.iterator_cf(self.ledger_cf, IteratorMode::Start);

        // Iterate through the CF_LEDGER column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.ledger_cf, key);
            })
            .collect::<Vec<_>>();

        self.clear_candidates()?;
        Ok(())
    }

    fn commit(self) -> Result<()> {
        if let Err(e) = self.inner.commit() {
            return Err(anyhow::Error::new(e).context("failed to commit"));
        }

        Ok(())
    }
}

impl<'db, DB: DBAccess> Mempool for DBTransaction<'db, DB> {
    fn add_tx(&self, tx: &ledger::Transaction) -> Result<()> {
        // Map Hash to serialized transaction
        let mut d = vec![];
        tx.write(&mut d)?;

        let hash = tx.hash();
        self.inner.put_cf(self.mempool_cf, hash, d)?;

        // Add Secondary indexes //
        // Nullifiers
        for n in tx.inner.inputs().into_iter() {
            let key: [u8; 32] = n.to_bytes().into();
            self.inner.put_cf(self.nullifiers_cf, key, vec![0])?;
        }

        // Map Fee_Hash to Null to facilitate sort-by-fee
        self.inner.put_cf(
            self.fees_cf,
            serialize_fee_key(tx.gas_price(), hash)?,
            vec![0],
        )?;

        Ok(())
    }

    fn get_tx(&self, hash: [u8; 32]) -> Result<Option<ledger::Transaction>> {
        let data = self.inner.get_cf(self.mempool_cf, hash)?;

        match data {
            // None has a meaning key not found
            None => Ok(None),
            Some(blob) => {
                Ok(Some(ledger::Transaction::read(&mut &blob.to_vec()[..])?))
            }
        }
    }

    fn get_tx_exists(&self, h: [u8; 32]) -> bool {
        // Check for hash if exists without deserializing
        self.snapshot.get_cf(self.mempool_cf, h).is_ok()
    }

    fn delete_tx(&self, h: [u8; 32]) -> Result<bool> {
        let tx = self.get_tx(h)?;
        if let Some(tx) = tx {
            let hash = tx.hash();

            self.inner.delete_cf(self.mempool_cf, hash)?;

            // Delete Secondary indexes
            // Delete Nullifiers
            for n in tx.inner.inputs().into_iter() {
                let key: [u8; 32] = n.to_bytes().into();
                self.inner.delete_cf(self.nullifiers_cf, key)?;
            }

            // Delete Fee_Hash
            self.inner.delete_cf(
                self.fees_cf,
                serialize_fee_key(tx.gas_price(), hash)?,
            )?;

            return Ok(true);
        }

        Ok(false)
    }

    fn get_any_nullifier_exists(&self, nullifiers: Vec<[u8; 32]>) -> bool {
        nullifiers
            .into_iter()
            .all(|n| self.snapshot.get_cf(self.nullifiers_cf, n).is_ok())
    }

    fn get_txs_sorted_by_fee(
        &self,
        max_gas_limit: u64,
    ) -> Result<Vec<Option<ledger::Transaction>>> {
        // The slippageGasLimit is the threshold that consider the "estimated
        // gas spent" acceptable even if it exceeds the strict GasLimit.
        // This is required to avoid to iterate the whole mempool until
        // it fit perfectly the block GasLimit
        let slippage_gas_limit = max_gas_limit + max_gas_limit / 10;

        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_last();

        let mut total_gas: u64 = 0;
        let mut txs_list = vec![];

        // Iterate all keys from the end in reverse lexicographic order
        while iter.valid() {
            if let Some(key) = iter.key() {
                let (read_gp, tx_hash) =
                    deserialize_fee_key(&mut &key.to_vec()[..])?;

                let mut tx = self.get_tx(tx_hash)?;
                if let Some(tx) = tx {
                    let gas_price = tx.gas_price();
                    debug_assert_eq!(read_gp, gas_price);

                    if let Some(res) = total_gas.checked_add(gas_price) {
                        total_gas = res;
                        if total_gas > slippage_gas_limit {
                            break;
                        }
                    } else {
                        break;
                    }

                    txs_list.push(Some(tx));
                }
            }

            iter.prev();
        }

        Ok(txs_list)
    }
}

fn serialize_fee_key(fee: u64, hash: [u8; 32]) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &fee.to_be_bytes())?;
    std::io::Write::write_all(&mut w, &hash)?;
    Ok(w)
}

fn deserialize_fee_key<R: Read>(r: &mut R) -> Result<(u64, [u8; 32])> {
    // Read fee
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let fee = u64::from_be_bytes(buf);

    // Read tx hash
    let mut hash = [0u8; 32];
    r.read_exact(&mut hash[..])?;

    Ok((fee, hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use node_data::ledger;

    use fake::{Dummy, Fake, Faker};
    use rand::prelude::*;
    use rand::Rng;

    #[test]
    fn test_store_block() {
        let t = TestWrapper {
            path: "_test_store_block",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());

            let b: ledger::Block = Faker.fake();
            let hash = b.header.hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert_eq!(
                    txn.fetch_block(&hash)?.unwrap().header.hash,
                    b.header.hash
                );
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
            let b: ledger::Block = Faker.fake();
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
            let mut b: ledger::Block = Faker.fake();
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
                assert_blocks_eq(&mut txn.fetch_block(&hash)?.unwrap(), &mut b);
                Ok(())
            });
        });
    }

    fn assert_blocks_eq(a: &mut ledger::Block, b: &mut ledger::Block) {
        assert!(a.calculate_hash().is_ok());
        assert!(b.calculate_hash().is_ok());
        assert!(a.header.hash.eq(&b.header.hash));
    }

    #[test]
    fn test_add_tx() {
        let t = TestWrapper {
            path: "test_add_tx",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let t: ledger::Transaction = Faker.fake();

            assert!(db.update(|txn| { txn.add_tx(&t) }).is_ok());

            db.view(|txn| {
                assert!(txn.get_tx_exists(t.hash()));

                let fetched_tx =
                    txn.get_tx(t.hash()).expect("valid contract call").unwrap();

                assert_eq!(
                    fetched_tx.hash(),
                    t.hash(),
                    "fetched transaction should be the same"
                );
                Ok(())
            });

            // Delete a contract call
            db.update(|txn| {
                assert!(txn.delete_tx(t.hash()).expect("valid tx"));
                Ok(())
            });
        });
    }

    #[test]
    fn test_tx_sorted_by_fee() {
        let t = TestWrapper {
            path: "test_tx_sorted_by_fee",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            // Populate mempool with N contract calls
            let mut rng = rand::thread_rng();
            db.update(|txn| {
                for i in 0..10u32 {
                    let t: ledger::Transaction = Faker.fake();
                    txn.add_tx(&t)?;
                }
                Ok(())
            });

            // Assert txs are retrieved in descending order sorted by fee
            let max_gas_limit = u64::MAX - u64::MAX / 10;
            db.view(|txn| {
                let txs = txn
                    .get_txs_sorted_by_fee(max_gas_limit)
                    .expect("should return all txs");

                assert!(!txs.is_empty());
                let mut last_fee = u64::MAX;
                for t in txs {
                    let fee = t.expect("valid tx").gas_price();
                    assert!(
                        fee <= last_fee,
                        "tx fees are not in decreasing order"
                    );

                    println!("fee: {}", fee);
                }

                Ok(())
            });
        });
    }

    #[test]
    fn test_max_gas_limit() {
        let t = TestWrapper {
            path: "test_block_size_limit",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());

            db.update(|txn| {
                for i in 0..10u32 {
                    let t = ledger::faker::gen_dummy_tx(i as u64);
                    txn.add_tx(&t)?;
                }
                Ok(())
            });

            let max_gas_limit: u32 = 9 + 8 + 7;
            db.view(|txn| {
                let txs = txn
                    .get_txs_sorted_by_fee(max_gas_limit as u64)
                    .expect("should return all txs");

                assert_eq!(txs.len(), 3);
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
            // Destroy/deletion of a database can happen only before creating
            let opts = Options::default();
            rocksdb_lib::DB::destroy(&opts, Path::new(&self.path));

            test_func(self.path);

            // Destroy/deletion of a database can happen only after dropping DB.
            let opts = Options::default();
            rocksdb_lib::DB::destroy(&opts, Path::new(&self.path));
        }
    }
}
