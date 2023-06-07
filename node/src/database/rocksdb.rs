// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Candidate, Ledger, Persist, Register, DB};
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

use std::io;
use std::io::Read;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::vec;
use tokio::io::AsyncWriteExt;

use tracing::info;

enum TxType {
    ReadWrite,
    ReadOnly,
}

const CF_LEDGER_HEADER: &str = "cf_ledger_header";
const CF_LEDGER_TXS: &str = "cf_ledger_txs";
const CF_LEDGER_HEIGHT: &str = "cf_ledger_height";
const CF_CANDIDATES: &str = "cf_candidates";
const CF_MEMPOOL: &str = "cf_mempool";
const CF_MEMPOOL_NULLIFIERS: &str = "cf_mempool_nullifiers";
const CF_MEMPOOL_FEES: &str = "cf_mempool_fees";

const MAX_MEMPOOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB
const REGISTER_KEY: &[u8; 8] = b"register";

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
            .cf_handle(CF_LEDGER_HEADER)
            .expect("ledger_header column family must exist");

        let ledger_txs_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_TXS)
            .expect("CF_LEDGER_TXS column family must exist");

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

        let ledger_height_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER_HEIGHT)
            .expect("CF_LEDGER_HEIGHT column family must exist");

        let snapshot = self.rocksdb.snapshot();

        DBTransaction::<'_, OptimisticTransactionDB> {
            inner,
            access_type,
            candidates_cf,
            ledger_cf,
            ledger_txs_cf,
            mempool_cf,
            nullifiers_cf,
            fees_cf,
            ledger_height_cf,
            snapshot,
        }
    }
}

impl DB for Backend {
    type P<'a> = DBTransaction<'a, OptimisticTransactionDB>;

    fn create_or_open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        info!(
            "Opening database in {:?}",
            path.as_ref().to_str().unwrap_or_default()
        );
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
            ColumnFamilyDescriptor::new(CF_LEDGER_HEADER, Options::default()),
            ColumnFamilyDescriptor::new(CF_LEDGER_TXS, Options::default()),
            ColumnFamilyDescriptor::new(CF_LEDGER_HEIGHT, Options::default()),
            ColumnFamilyDescriptor::new(CF_CANDIDATES, Options::default()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_NULLIFIERS, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_FEES, mp_opts),
        ];

        Self {
            rocksdb: Arc::new(
                rocksdb_lib::OptimisticTransactionDB::open_cf_descriptors(
                    &opts, path, cfs,
                )
                .expect("should be a valid database in {path}"),
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

    // TODO: pack all column families into a single array
    // Candidates column family
    candidates_cf: &'db ColumnFamily,

    // Ledger column families
    ledger_cf: &'db ColumnFamily,
    ledger_txs_cf: &'db ColumnFamily,
    ledger_height_cf: &'db ColumnFamily,

    // Mempool column families
    mempool_cf: &'db ColumnFamily,
    nullifiers_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    snapshot: SnapshotWithThreadMode<'db, DB>,
}

impl<'db, DB: DBAccess> Ledger for DBTransaction<'db, DB> {
    fn store_block(&self, b: &ledger::Block, persisted: bool) -> Result<()> {
        // COLUMN FAMILY: CF_LEDGER_HEADER
        // It consists of one record per a block - Header record
        // It also includes single record to store metadata - Register record
        {
            let cf = self.ledger_cf;

            let mut buf = vec![];
            HeaderRecord {
                header: b.header.clone(),
                transactions_ids: b
                    .txs
                    .iter()
                    .map(|t| t.hash())
                    .collect::<Vec<[u8; 32]>>(),
            }
            .write(&mut buf);

            self.inner.put_cf(cf, b.header.hash, buf)?;

            // Overwrite the Register record
            let mut buf = vec![];
            Register {
                mrb_hash: b.header.hash,
                state_hash: b.header.state_hash,
            }
            .write(&mut buf)?;
            self.inner.put_cf(cf, REGISTER_KEY, buf)?;
        }

        // COLUMN FAMILY: CF_LEDGER_TXS
        {
            let cf = self.ledger_txs_cf;

            // store all block transactions
            for tx in &b.txs {
                let mut d = vec![];
                tx.write(&mut d)?;
                self.inner.put_cf(cf, tx.hash(), d)?;
            }
        }

        // CF: HEIGHT
        // Relation: Map block height to block hash
        self.inner.put_cf(
            self.ledger_height_cf,
            b.header.height.to_le_bytes(),
            b.header.hash,
        )?;

        Ok(())
    }

    fn delete_block(&self, b: &ledger::Block) -> Result<()> {
        for tx in &b.txs {
            self.inner.delete_cf(self.ledger_txs_cf, tx.hash())?;
        }

        let key = b.header.hash;
        self.inner.delete_cf(self.ledger_cf, key)?;
        self.inner.delete_cf(self.ledger_cf, REGISTER_KEY)?;

        self.inner
            .delete_cf(self.ledger_height_cf, b.header.height.to_le_bytes())?;

        Ok(())
    }

    fn get_block_exists(&self, hash: &[u8]) -> Result<bool> {
        Ok(self.snapshot.get_cf(self.ledger_cf, hash)?.is_some())
    }

    fn fetch_block(&self, hash: &[u8]) -> Result<Option<ledger::Block>> {
        match self.snapshot.get_cf(self.ledger_cf, hash)? {
            Some(blob) => {
                let record = HeaderRecord::read(&mut &blob[..])?;

                // Retrieve all transactions buffers with single call
                let txs_buffers = self.snapshot.multi_get_cf(
                    record
                        .transactions_ids
                        .iter()
                        .map(|id| (self.ledger_txs_cf, id))
                        .collect::<Vec<(&ColumnFamily, &[u8; 32])>>(),
                );

                let mut txs = vec![];
                for buf in txs_buffers {
                    let mut buf = buf?.unwrap();
                    let tx = ledger::Transaction::read(&mut &buf.to_vec()[..])?;
                    txs.push(tx);
                }

                Ok(Some(ledger::Block {
                    header: record.header,
                    txs,
                }))
            }
            None => Ok(None),
        }
    }

    fn fetch_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<[u8; 32]>> {
        Ok(self
            .snapshot
            .get_cf(self.ledger_height_cf, height.to_le_bytes())?
            .map(|h| {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(h.as_slice());
                hash
            }))
    }

    fn get_ledger_tx_by_hash(
        &self,
        tx_hash: &[u8],
    ) -> Result<Option<ledger::Transaction>> {
        let tx = self
            .snapshot
            .get_cf(self.ledger_txs_cf, tx_hash)?
            .map(|blob| ledger::Transaction::read(&mut &blob[..]))
            .transpose()?;

        Ok(tx)
    }

    /// Returns true if the transaction exists in the
    /// ledger
    ///
    /// This is a convenience method that checks if a transaction exists in the
    /// ledger without unmarshalling the transaction
    fn get_ledger_tx_exists(&self, tx_hash: &[u8]) -> Result<bool> {
        Ok(self.snapshot.get_cf(self.ledger_txs_cf, tx_hash)?.is_some())
    }

    /// Returns stored register data
    fn get_register(&self) -> Result<Option<Register>> {
        if let Some(mut data) =
            self.snapshot.get_cf(self.ledger_cf, REGISTER_KEY)?
        {
            return Ok(Some(Register::read(&mut &data[..])?));
        }

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
        for n in tx.inner.inputs().iter() {
            let key = n.to_bytes();
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

    fn get_tx_exists(&self, h: [u8; 32]) -> Result<bool> {
        Ok(self.snapshot.get_cf(self.mempool_cf, h)?.is_some())
    }

    fn delete_tx(&self, h: [u8; 32]) -> Result<bool> {
        let tx = self.get_tx(h)?;
        if let Some(tx) = tx {
            let hash = tx.hash();

            self.inner.delete_cf(self.mempool_cf, hash)?;

            // Delete Secondary indexes
            // Delete Nullifiers
            for n in tx.inner.inputs().iter() {
                let key = n.to_bytes();
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
        nullifiers.into_iter().any(|n| {
            let r = self.snapshot.get_cf(self.nullifiers_cf, n);
            match r {
                Ok(r) => r.is_some(),
                _ => false,
            }
        })
    }

    fn get_txs_sorted_by_fee(
        &self,
        max_gas_limit: u64,
    ) -> Result<Vec<ledger::Transaction>> {
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

                    txs_list.push(tx);
                } else {
                    tracing::error!("get_txs_sorted_by_fee tx: not found");
                }
            }

            iter.prev();
        }

        Ok(txs_list)
    }

    fn get_txs_hashes(&self) -> Result<Vec<[u8; 32]>> {
        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_last();

        let mut txs_list = vec![];
        while iter.valid() {
            if let Some(key) = iter.key() {
                let (read_gp, tx_hash) =
                    deserialize_fee_key(&mut &key.to_vec()[..])?;

                txs_list.push(tx_hash);
            }

            iter.prev();
        }

        Ok(txs_list)
    }
}

impl<'db, DB: DBAccess> std::fmt::Debug for DBTransaction<'db, DB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //  Print ledger blocks
        let iter = self.inner.iterator_cf(self.ledger_cf, IteratorMode::Start);

        iter.map(Result::unwrap).try_for_each(|(hash, _)| {
            if let Ok(Some(blob)) =
                self.snapshot.get_cf(self.ledger_cf, &hash[..])
            {
                let b = ledger::Block::read(&mut &blob[..]).unwrap_or_default();
                writeln!(f, "ledger_block [{}]: {:#?}", b.header.height, b)
            } else {
                Ok(())
            }
        })?;

        // Print candidate blocks
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, IteratorMode::Start);

        let results: std::fmt::Result =
            iter.map(Result::unwrap).try_for_each(|(hash, _)| {
                if let Ok(Some(blob)) =
                    self.snapshot.get_cf(self.candidates_cf, &hash[..])
                {
                    let b =
                        ledger::Block::read(&mut &blob[..]).unwrap_or_default();
                    writeln!(
                        f,
                        "candidate_block [{}]: {:#?}",
                        b.header.height, b
                    )
                } else {
                    Ok(())
                }
            });

        results
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

struct HeaderRecord {
    header: ledger::Header,
    transactions_ids: Vec<[u8; 32]>,
}

impl Serializable for HeaderRecord {
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

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read block header
        let header = ledger::Header::read(r)?;

        // Read transactions count
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;

        let len = u32::from_le_bytes(buf);

        // Read transactions hashes
        let mut transactions_ids = vec![];
        for pos in 0..len {
            let mut tx_id = [0u8; 32];
            r.read_exact(&mut tx_id[..])?;

            transactions_ids.push(tx_id);
        }

        Ok(Self {
            header,
            transactions_ids,
        })
    }
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
            let db: Backend = Backend::create_or_open(path);

            let b: ledger::Block = Faker.fake();
            assert!(b.txs.len() > 0);

            let hash = b.header.hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                // Assert block header is fully fetched from ledger
                let db_blk = txn.fetch_block(&hash)?.unwrap();
                assert_eq!(db_blk.header.hash, b.header.hash);

                // Assert all transactions are fully fetched from ledger as
                // well.
                for pos in (0..b.txs.len()) {
                    assert_eq!(db_blk.txs[pos].hash(), b.txs[pos].hash());
                }

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
            let db: Backend = Backend::create_or_open(path);
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
            let db: Backend = Backend::create_or_open(path);
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
    fn test_add_mempool_tx() {
        let t = TestWrapper {
            path: "test_add_tx",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let t: ledger::Transaction = Faker.fake();

            assert!(db.update(|txn| { txn.add_tx(&t) }).is_ok());

            db.view(|vq| {
                assert!(Mempool::get_tx_exists(&vq, t.hash()).unwrap());

                let fetched_tx =
                    vq.get_tx(t.hash()).expect("valid contract call").unwrap();

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
    fn test_mempool_txs_sorted_by_fee() {
        let t = TestWrapper {
            path: "test_mempool_txs_sorted_by_fee",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path);
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
                    let fee = t.gas_price();
                    assert!(
                        fee <= last_fee,
                        "tx fees are not in decreasing order"
                    );
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
            let db: Backend = Backend::create_or_open(path);

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

    #[test]
    fn test_get_ledger_tx_by_hash() {
        let t = TestWrapper {
            path: "test_get_ledger_tx_by_hash",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let mut b: ledger::Block = Faker.fake();
            assert!(b.txs.len() > 0);

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            // Assert all transactions of the accepted (stored) block are
            // accessible by hash.
            db.view(|v| {
                for t in b.txs.iter() {
                    assert!(v
                        .get_ledger_tx_by_hash(&t.hash())
                        .expect("should not return error")
                        .expect("should find a transaction")
                        .eq(&t));
                }

                Ok(())
            });
        });
    }

    #[test]
    fn test_fetch_block_hash_by_height() {
        let t = TestWrapper {
            path: "test_fetch_block_hash_by_height",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let mut b: ledger::Block = Faker.fake();

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(v
                    .fetch_block_hash_by_height(b.header.height)
                    .expect("should not return error")
                    .expect("should find a block")
                    .eq(&b.header.hash));

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
