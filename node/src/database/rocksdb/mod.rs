// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::{io, vec};

use anyhow::Result;
use dusk_core::transfer::data::BlobSidecar;
use node_data::Serializable;
use node_data::ledger::{
    Block, Fault, Header, Label, LedgerTransaction, SpendingId,
    SpentTransaction,
};
use node_data::message::{ConsensusHeader, payload};
use rocksdb::{
    AsColumnFamilyRef, BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor,
    DBAccess, DBRawIteratorWithThreadMode, IteratorMode, LogLevel,
    OptimisticTransactionDB, OptimisticTransactionOptions, Options,
    WriteOptions,
};
use tracing::info;

use super::{
    ConsensusStorage, DB, DatabaseOptions, Ledger, LightBlock, Metadata,
    Persist,
};
use crate::database::Mempool;

const CF_LEDGER_HEADER: &str = "cf_ledger_header";
const CF_LEDGER_TXS: &str = "cf_ledger_txs";
const CF_LEDGER_BLOBS: &str = "cf_ledger_blobs";
const CF_LEDGER_BLOBS_HEIGHT: &str = "cf_ledger_blobs_height";
const CF_LEDGER_FAULTS: &str = "cf_ledger_faults";
const CF_LEDGER_HEIGHT: &str = "cf_ledger_height";
const CF_CANDIDATES: &str = "cf_candidates";
const CF_CANDIDATES_HEIGHT: &str = "cf_candidates_height";
const CF_VALIDATION_RESULTS: &str = "cf_validation_results";
const CF_MEMPOOL: &str = "cf_mempool";
const CF_MEMPOOL_SPENDING_ID: &str = "cf_mempool_spending_id";
const CF_MEMPOOL_FEES: &str = "cf_mempool_fees";
const CF_METADATA: &str = "cf_metadata";

const DB_FOLDER_NAME: &str = "chain.db";

// List of supported metadata keys
pub const MD_HASH_KEY: &[u8] = b"hash_key";
pub const MD_STATE_ROOT_KEY: &[u8] = b"state_hash_key";
pub const MD_AVG_VALIDATION: &[u8] = b"avg_validation_time";
pub const MD_AVG_RATIFICATION: &[u8] = b"avg_ratification_time";
pub const MD_AVG_PROPOSAL: &[u8] = b"avg_proposal_time";
pub const MD_LAST_ITER: &[u8] = b"consensus_last_iter";

#[derive(Clone)]
pub struct Backend {
    rocksdb: Arc<OptimisticTransactionDB>,
}

impl Backend {
    fn begin_tx(&self) -> DBTransaction<'_, OptimisticTransactionDB> {
        // Create a new RocksDB transaction
        let write_options = WriteOptions::default();
        let tx_options = OptimisticTransactionOptions::default();

        let inner = self.rocksdb.transaction_opt(&write_options, &tx_options);

        // Borrow column families
        let ledger_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_HEADER)
            .expect("ledger_header column family must exist");

        let ledger_txs_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_TXS)
            .expect("CF_LEDGER_TXS column family must exist");

        let ledger_faults_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_FAULTS)
            .expect("CF_LEDGER_FAULTS column family must exist");

        let candidates_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES)
            .expect("candidates column family must exist");

        let candidates_height_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES_HEIGHT)
            .expect("candidates_height column family must exist");

        let validation_results_cf = self
            .rocksdb
            .cf_handle(CF_VALIDATION_RESULTS)
            .expect("validation result column family must exist");

        let mempool_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL)
            .expect("mempool column family must exist");

        let spending_id_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_SPENDING_ID)
            .expect("CF_MEMPOOL_SPENDING_ID column family must exist");

        let fees_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_FEES)
            .expect("CF_MEMPOOL_FEES column family must exist");

        let ledger_height_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_HEIGHT)
            .expect("CF_LEDGER_HEIGHT column family must exist");

        let metadata_cf = self
            .rocksdb
            .cf_handle(CF_METADATA)
            .expect("CF_METADATA column family must exist");

        let ledger_blobs_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_BLOBS)
            .expect("CF_LEDGER_BLOBS column family must exist");

        let ledger_blobs_height_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_BLOBS_HEIGHT)
            .expect("CF_LEDGER_BLOBS_HEIGHT column family must exist");

        DBTransaction::<'_, OptimisticTransactionDB> {
            inner,
            candidates_cf,
            candidates_height_cf,
            validation_results_cf,
            ledger_cf,
            ledger_txs_cf,
            ledger_faults_cf,
            mempool_cf,
            spending_id_cf,
            fees_cf,
            ledger_height_cf,
            ledger_blobs_cf,
            ledger_blobs_height_cf,
            metadata_cf,
            cumulative_inner_size: RefCell::new(0),
        }
    }
}

impl DB for Backend {
    type P<'a> = DBTransaction<'a, OptimisticTransactionDB>;

    fn create_or_open<T>(path: T, db_opts: DatabaseOptions) -> Self
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().join(DB_FOLDER_NAME);
        info!("Opening database in {path:?}, {:?} ", db_opts);

        // A set of options for initializing any blocks-related CF (including
        // METADATA CF)
        let mut blocks_cf_opts = Options::default();
        blocks_cf_opts.create_if_missing(db_opts.create_if_missing);
        blocks_cf_opts.create_missing_column_families(true);
        blocks_cf_opts.set_level_compaction_dynamic_level_bytes(true);
        blocks_cf_opts
            .set_write_buffer_size(db_opts.blocks_cf_max_write_buffer_size);

        if db_opts.enable_debug {
            blocks_cf_opts.set_log_level(LogLevel::Info);
            blocks_cf_opts.set_dump_malloc_stats(true);
            blocks_cf_opts.enable_statistics();
        }

        if db_opts.blocks_cf_disable_block_cache {
            let mut block_opts = BlockBasedOptions::default();
            block_opts.disable_cache();
            blocks_cf_opts.set_block_based_table_factory(&block_opts);
        }

        // Configure CF_MEMPOOL column family, so it benefits from low
        // write-latency of L0
        let mut mp_opts = blocks_cf_opts.clone();
        // Disable WAL by default
        mp_opts.set_manual_wal_flush(true);
        mp_opts.create_if_missing(true);
        mp_opts.create_missing_column_families(true);
        mp_opts.set_write_buffer_size(db_opts.mempool_cf_max_write_buffer_size);

        if db_opts.enable_debug {
            mp_opts.set_log_level(LogLevel::Info);
            mp_opts.set_dump_malloc_stats(true);
            mp_opts.enable_statistics();
        }

        let cfs = vec![
            ColumnFamilyDescriptor::new(
                CF_LEDGER_HEADER,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_LEDGER_TXS, blocks_cf_opts.clone()),
            ColumnFamilyDescriptor::new(
                CF_LEDGER_FAULTS,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_LEDGER_HEIGHT,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_LEDGER_BLOBS,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_LEDGER_BLOBS_HEIGHT,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_CANDIDATES, blocks_cf_opts.clone()),
            ColumnFamilyDescriptor::new(
                CF_CANDIDATES_HEIGHT,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_VALIDATION_RESULTS,
                blocks_cf_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_METADATA, blocks_cf_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL, mp_opts.clone()),
            ColumnFamilyDescriptor::new(
                CF_MEMPOOL_SPENDING_ID,
                mp_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_FEES, mp_opts.clone()),
        ];

        Self {
            rocksdb: Arc::new(
                OptimisticTransactionDB::open_cf_descriptors(
                    &blocks_cf_opts,
                    &path,
                    cfs,
                )
                .unwrap_or_else(|_| {
                    panic!("should be a valid database in {path:?}")
                }),
            ),
        }
    }

    fn view<F, T>(&self, f: F) -> T
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> T,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx();

        // Execute all read-only transactions in isolation
        let ret = f(&tx);
        tx.rollback().expect("rollback to succeed for readonly");
        ret
    }

    fn update<F, T>(&self, execute: F) -> Result<T>
    where
        F: for<'a> FnOnce(&mut Self::P<'a>) -> Result<T>,
    {
        self.update_dry_run(false, execute)
    }

    fn update_dry_run<F, T>(&self, dry_run: bool, execute: F) -> Result<T>
    where
        F: for<'a> FnOnce(&mut Self::P<'a>) -> Result<T>,
    {
        // Create read-write transaction
        let mut tx = self.begin_tx();

        // If f returns err, no commit will be applied into backend
        // storage
        let ret = execute(&mut tx)?;

        if dry_run {
            tx.rollback()?;
        } else {
            // Apply changes in atomic way
            tx.commit()?;
        }

        Ok(ret)
    }

    fn close(&mut self) {}
}

pub struct DBTransaction<'db, DB: DBAccess> {
    inner: rocksdb::Transaction<'db, DB>,
    /// cumulative size of transaction footprint
    cumulative_inner_size: RefCell<usize>,

    // TODO: pack all column families into a single array
    // Candidates column family
    candidates_cf: &'db ColumnFamily,
    candidates_height_cf: &'db ColumnFamily,
    // ValidationResults column family
    validation_results_cf: &'db ColumnFamily,

    // Ledger column families
    ledger_cf: &'db ColumnFamily,
    ledger_faults_cf: &'db ColumnFamily,
    ledger_txs_cf: &'db ColumnFamily,
    ledger_height_cf: &'db ColumnFamily,
    ledger_blobs_cf: &'db ColumnFamily,
    ledger_blobs_height_cf: &'db ColumnFamily,

    // Mempool column families
    mempool_cf: &'db ColumnFamily,
    spending_id_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    metadata_cf: &'db ColumnFamily,
}

mod blocks;
mod error;
mod metadata_indexes;
mod tx_events;
mod tx_utils;

use metadata_indexes::{
    deserialize_iter_key, deserialize_key, serialize_iter_key, serialize_key,
};

impl node_data::Serializable for LightBlock {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        // Write block header
        self.header.write(w)?;

        // Write transactions count
        let len = self.transactions_ids.len() as u32;
        w.write_all(&len.to_le_bytes())?;

        // Write transactions hashes
        for tx_id in &self.transactions_ids {
            w.write_all(tx_id)?;
        }

        // Write faults count
        let len = self.faults_ids.len() as u32;
        w.write_all(&len.to_le_bytes())?;

        // Write faults id
        for f_id in &self.faults_ids {
            w.write_all(f_id)?;
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read block header
        let header = Header::read(r)?;

        // Read transactions count
        let len = Self::read_u32_le(r)?;

        // Read transactions hashes
        let mut transactions_ids = vec![];
        for _ in 0..len {
            let mut tx_id = [0u8; 32];
            r.read_exact(&mut tx_id[..])?;

            transactions_ids.push(tx_id);
        }

        // Read faults count
        let len = Self::read_u32_le(r)?;

        // Read faults ids
        let mut faults_ids = vec![];
        for _ in 0..len {
            let mut f_id = [0u8; 32];
            r.read_exact(&mut f_id[..])?;

            faults_ids.push(f_id);
        }

        Ok(Self {
            header,
            transactions_ids,
            faults_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use fake::{Fake, Faker};
    use node_data::{hard_fork, ledger};

    use super::*;

    #[test]
    fn test_store_block() {
        TestWrapper::new("test_store_block").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());

            let b: Block = Faker.fake();
            assert!(!b.txs().is_empty());
            let spent_txs = to_spent_txs(b.header().height, b.txs());

            let hash = b.header().hash;

            assert!(
                db.update(|txn| {
                    txn.store_block(
                        b.header(),
                        &spent_txs,
                        b.faults(),
                        Label::Final(3),
                    )?;
                    Ok(())
                })
                .is_ok()
            );

            db.view(|txn| {
                // Assert block header is fully fetched from ledger
                let db_blk = txn
                    .block(&hash)
                    .expect("Block to be fetched")
                    .expect("Block to exist");
                assert_eq!(db_blk.header().hash, b.header().hash);

                // Assert all transactions are fully fetched from ledger as
                // well.
                for pos in 0..b.txs().len() {
                    assert_eq!(db_blk.txs()[pos], spent_txs[pos].inner);
                }

                // Assert all faults are fully fetched from ledger as
                // well.
                for pos in 0..b.faults().len() {
                    assert_eq!(db_blk.faults()[pos].id(), b.faults()[pos].id());
                }
            });

            assert!(
                db.update(|txn| {
                    txn.clear_database()?;
                    Ok(())
                })
                .is_ok()
            );

            db.view(|txn| {
                assert!(
                    txn.block(&hash).expect("block to be fetched").is_none()
                );
            });
        });
    }

    #[test]
    fn test_read_only() {
        TestWrapper::new("test_read_only").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            let spent_txs = to_spent_txs(b.header().height, b.txs());
            db.update_dry_run(true, |txn| {
                txn.store_block(
                    b.header(),
                    &spent_txs,
                    b.faults(),
                    Label::Final(3),
                )
            })
            .expect("block to be stored");
            db.view(|txn| {
                assert!(
                    txn.block(&b.header().hash)
                        .expect("block to be fetched")
                        .is_none()
                );
            });
        });
    }

    #[test]
    fn test_transaction_isolation() {
        TestWrapper::new("test_transaction_isolation").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let mut b: Block = Faker.fake();
            let spent_txs = to_spent_txs(b.header().height, b.txs());
            let hash = b.header().hash;

            db.view(|txn| {
                // Simulate a concurrent update is committed during read-only
                // transaction
                assert!(
                    db.update(|inner| {
                        inner
                            .store_block(
                                b.header(),
                                &spent_txs,
                                b.faults(),
                                Label::Final(3),
                            )
                            .unwrap();

                        // We support Read-Your-Own-Writes
                        assert!(inner.block(&hash)?.is_some());
                        // Data is isolated until the transaction is not
                        // committed
                        assert!(txn.block(&hash)?.is_none());
                        Ok(())
                    })
                    .is_ok()
                );

                // Asserts that the read-only/view transaction get the updated
                // data after the tx is committed
                assert!(
                    txn.block(&hash).expect("block to be fetched").is_some()
                );
            });

            // Asserts that update was done
            db.view(|txn| {
                assert_blocks_eq(
                    &mut txn
                        .block(&hash)
                        .expect("block to be fetched")
                        .unwrap(),
                    &mut b,
                );
            });
        });
    }

    fn assert_blocks_eq(a: &Block, b: &Block) {
        assert!(a.header().hash != [0u8; 32]);
        assert!(a.header().hash.eq(&b.header().hash));
    }

    #[test]
    fn test_add_mempool_tx() {
        TestWrapper::new("test_add_tx").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let t: LedgerTransaction = Faker.fake();

            assert!(db.update(|txn| { txn.store_mempool_tx(&t, 0) }).is_ok());

            db.view(|vq| {
                assert!(vq.mempool_tx_exists(t.id()).unwrap());

                let fetched_tx = vq
                    .mempool_tx(t.id())
                    .expect("valid contract call")
                    .unwrap();

                assert_eq!(
                    fetched_tx.id(),
                    t.id(),
                    "fetched transaction should be the same"
                );
            });

            // Delete a contract call
            db.update(|txn| {
                let deleted =
                    txn.delete_mempool_tx(t.id(), false).expect("valid tx");
                assert!(deleted.len() == 1);
                Ok(())
            })
            .unwrap();
        });
    }

    #[test]
    fn test_mempool_txs_sorted_by_fee() {
        TestWrapper::new("test_mempool_txs_sorted_by_fee").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            // Populate mempool with N contract calls
            let _rng = rand::thread_rng();
            db.update(|txn| {
                for _i in 0..10u32 {
                    let t: LedgerTransaction = Faker.fake();
                    txn.store_mempool_tx(&t, 0)?;
                }
                Ok(())
            })
            .unwrap();

            db.view(|txn| {
                let txs = txn.mempool_txs_sorted_by_fee();

                let mut last_fee = u64::MAX;
                for t in txs {
                    let fee = t.gas_price();
                    assert!(
                        fee <= last_fee,
                        "tx fees are not in decreasing order"
                    );
                    last_fee = fee
                }
                assert_ne!(last_fee, u64::MAX, "No tx has been processed")
            });
        });
    }

    #[test]
    fn test_txs_count() {
        TestWrapper::new("test_txs_count").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());

            const N: usize = 100;
            const D: usize = 50;

            let txs: Vec<_> = (0..N)
                .map(|i| ledger::faker::gen_dummy_tx(i as u64))
                .collect();

            db.update(|db| {
                assert_eq!(db.mempool_txs_count(), 0);
                txs.iter().for_each(|t| {
                    db.store_mempool_tx(&t, 0).expect("tx should be added")
                });
                Ok(())
            })
            .unwrap();

            db.update(|db| {
                // Ensure txs count is equal to the number of added tx
                assert_eq!(db.mempool_txs_count(), N);

                txs.iter().take(D).for_each(|tx| {
                    let deleted = db
                        .delete_mempool_tx(tx.id(), false)
                        .expect("transaction should be deleted");
                    assert!(deleted.len() == 1);
                });

                Ok(())
            })
            .unwrap();

            // Ensure txs count is updated after the deletion
            db.update(|db| {
                assert_eq!(db.mempool_txs_count(), N - D);
                Ok(())
            })
            .unwrap();
        });
    }

    #[test]
    fn test_max_gas_limit() {
        TestWrapper::new("test_block_size_limit").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());

            db.update(|txn| {
                for i in 0..10u32 {
                    let t = ledger::faker::gen_dummy_tx(i as u64);
                    txn.store_mempool_tx(&t, 0)?;
                }
                Ok(())
            })
            .unwrap();

            let total_gas_price: u64 = 9 + 8 + 7 + 6 + 5 + 4 + 3 + 2 + 1;
            db.view(|txn| {
                let txs = txn
                    .mempool_txs_sorted_by_fee()
                    .map(|t| t.gas_price())
                    .sum::<u64>();

                assert_eq!(txs, total_gas_price);
            });
        });
    }

    #[test]
    fn test_get_expired_txs() {
        TestWrapper::new("test_get_expired_txs").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());

            let mut expiry_list = HashSet::new();
            let _ = db.update(|txn| {
                (1..101).for_each(|i| {
                    let t = ledger::faker::gen_dummy_tx(i as u64);
                    txn.store_mempool_tx(&t, i).expect("tx should be added");
                    expiry_list.insert(t.id());
                });

                (1000..1100).for_each(|i| {
                    let t = ledger::faker::gen_dummy_tx(i as u64);
                    txn.store_mempool_tx(&t, i).expect("tx should be added");
                });

                Ok(())
            });

            db.view(|vq| {
                let expired: HashSet<_> = vq
                    .mempool_expired_txs(100)
                    .unwrap()
                    .into_iter()
                    .map(|id| id)
                    .collect();

                assert_eq!(expiry_list, expired);
            });
        });
    }

    fn to_spent_txs(
        block_height: u64,
        txs: &[LedgerTransaction],
    ) -> Vec<SpentTransaction> {
        let format = hard_fork::ledger_tx_format_at(block_height);
        txs.iter()
            .map(|t| SpentTransaction {
                inner: LedgerTransaction::from_protocol_with_format(
                    t.protocol().clone(),
                    format,
                )
                .expect("test tx should canonicalize"),
                block_height,
                gas_spent: 0,
                err: None,
            })
            .collect()
    }

    #[test]
    fn test_get_ledger_tx_by_hash() {
        TestWrapper::new("test_get_ledger_tx_by_hash").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            assert!(!b.txs().is_empty());
            let spent_txs = to_spent_txs(b.header().height, b.txs());

            // Store a block
            assert!(
                db.update(|txn| {
                    txn.store_block(
                        b.header(),
                        &spent_txs,
                        b.faults(),
                        Label::Final(3),
                    )?;
                    Ok(())
                })
                .is_ok()
            );

            // Assert all transactions of the accepted (stored) block are
            // accessible by hash.
            db.view(|v| {
                for expected in &spent_txs {
                    let fetched = v
                        .ledger_tx(&expected.inner.id())
                        .expect("should not return error")
                        .expect("should find a transaction");
                    assert_eq!(fetched.inner, expected.inner);
                    assert_eq!(fetched.block_height, expected.block_height);
                }
            });
        });
    }

    #[test]
    fn test_fetch_block_hash_by_height() {
        TestWrapper::new("test_fetch_block_hash_by_height").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            let spent_txs = to_spent_txs(b.header().height, b.txs());

            // Store a block
            assert!(
                db.update(|txn| {
                    txn.store_block(
                        b.header(),
                        &spent_txs,
                        b.faults(),
                        Label::Attested(3),
                    )?;
                    Ok(())
                })
                .is_ok()
            );

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(
                    v.block_hash_by_height(b.header().height)
                        .expect("should not return error")
                        .expect("should find a block")
                        .eq(&b.header().hash)
                );
            });
        });
    }

    #[test]
    fn test_fetch_block_label_by_height() {
        TestWrapper::new("test_fetch_block_hash_by_height").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            let spent_txs = to_spent_txs(b.header().height, b.txs());

            // Store a block
            assert!(
                db.update(|txn| {
                    txn.store_block(
                        b.header(),
                        &spent_txs,
                        b.faults(),
                        Label::Attested(3),
                    )?;
                    Ok(())
                })
                .is_ok()
            );

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(
                    v.block_label_by_height(b.header().height)
                        .expect("should not return error")
                        .expect("should find a block")
                        .1
                        .eq(&Label::Attested(3))
                );
            });
        });
    }

    #[test]
    /// Ensures delete_block fn removes all keys of a single block
    fn test_delete_block() {
        let t = TestWrapper::new("test_fetch_block_hash_by_height");
        t.run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: ledger::Block = Faker.fake();
            let spent_txs = to_spent_txs(b.header().height, b.txs());

            assert!(
                db.update(|ut| {
                    ut.store_block(
                        b.header(),
                        &spent_txs,
                        b.faults(),
                        Label::Final(3),
                    )?;
                    Ok(())
                })
                .is_ok()
            );

            assert!(
                db.update(|ut| {
                    ut.delete_block(&b)?;
                    Ok(())
                })
                .is_ok()
            );
        });

        let path = t.get_path();
        let opts = Options::default();

        let vec = rocksdb::DB::list_cf(&opts, &path).unwrap();
        assert!(!vec.is_empty());

        // Ensure no block fields leak after its deletion
        let db = rocksdb::DB::open_cf(&opts, &path, vec.clone()).unwrap();
        vec.into_iter()
            .map(|cf_name| {
                if cf_name == CF_METADATA {
                    return;
                }

                let cf = db.cf_handle(&cf_name).unwrap();
                assert_eq!(
                    db.iterator_cf(cf, IteratorMode::Start)
                        .map(Result::unwrap)
                        .count(),
                    0
                );
            })
            .for_each(drop);
    }

    struct TestWrapper(tempfile::TempDir);

    impl TestWrapper {
        fn new(path: &'static str) -> Self {
            Self(
                tempfile::TempDir::with_prefix(path)
                    .expect("Temp directory to be created"),
            )
        }

        pub fn run<F>(&self, test_func: F)
        where
            F: FnOnce(&Path),
        {
            test_func(self.0.path());
        }

        pub fn get_path(&self) -> std::path::PathBuf {
            self.0.path().to_owned().join(DB_FOLDER_NAME)
        }
    }
}
