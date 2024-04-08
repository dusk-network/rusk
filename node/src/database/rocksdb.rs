// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Candidate, Ledger, Metadata, Persist, DB};
use anyhow::Result;
use std::cell::RefCell;

use node_data::ledger::{self, Label, SpentTransaction};
use node_data::Serializable;

use crate::database::Mempool;

use rocksdb_lib::{
    AsColumnFamilyRef, ColumnFamily, ColumnFamilyDescriptor, DBAccess,
    DBRawIteratorWithThreadMode, IteratorMode, OptimisticTransactionDB,
    OptimisticTransactionOptions, Options, SnapshotWithThreadMode, Transaction,
    WriteOptions,
};

use std::collections::HashSet;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::vec;

use tracing::info;

const CF_LEDGER_HEADER: &str = "cf_ledger_header";
const CF_LEDGER_TXS: &str = "cf_ledger_txs";
const CF_LEDGER_HEIGHT: &str = "cf_ledger_height";
const CF_CANDIDATES: &str = "cf_candidates";
const CF_CANDIDATES_HEIGHT: &str = "cf_candidates_height";
const CF_MEMPOOL: &str = "cf_mempool";
const CF_MEMPOOL_NULLIFIERS: &str = "cf_mempool_nullifiers";
const CF_MEMPOOL_FEES: &str = "cf_mempool_fees";
const CF_METADATA: &str = "cf_metadata";
const MAX_MEMPOOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

const DB_FOLDER_NAME: &str = "chain.db";

// List of supported metadata keys
pub const MD_HASH_KEY: &[u8] = b"hash_key";
pub const MD_STATE_ROOT_KEY: &[u8] = b"state_hash_key";
pub const MD_AVG_VALIDATION: &[u8] = b"avg_validation_time";
pub const MD_AVG_RATIFICATION: &[u8] = b"avg_ratification_time";
pub const MD_AVG_PROPOSAL: &[u8] = b"avg_proposal_time";

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

        let candidates_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES)
            .expect("candidates column family must exist");

        let candidates_height_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES_HEIGHT)
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

        let metadata_cf = self
            .rocksdb
            .cf_handle(CF_METADATA)
            .expect("CF_METADATA column family must exist");

        let snapshot = self.rocksdb.snapshot();

        DBTransaction::<'_, OptimisticTransactionDB> {
            inner,
            candidates_cf,
            candidates_height_cf,
            ledger_cf,
            ledger_txs_cf,
            mempool_cf,
            nullifiers_cf,
            fees_cf,
            ledger_height_cf,
            metadata_cf,
            snapshot,
            cumulative_inner_size: RefCell::new(0),
        }
    }
}

impl DB for Backend {
    type P<'a> = DBTransaction<'a, OptimisticTransactionDB>;

    fn create_or_open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().join(DB_FOLDER_NAME);
        info!("Opening database in {path:?}");

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
            ColumnFamilyDescriptor::new(
                CF_CANDIDATES_HEIGHT,
                Options::default(),
            ),
            ColumnFamilyDescriptor::new(CF_MEMPOOL, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_NULLIFIERS, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_FEES, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_METADATA, mp_opts),
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

    fn view<F, T>(&self, f: F) -> T
    where
        F: for<'a> FnOnce(Self::P<'a>) -> T,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx();

        // Execute all read-only transactions in isolation
        f(tx)
    }

    fn update<F, T>(&self, execute: F) -> Result<T>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<T>,
    {
        // Create read-write transaction
        let tx = self.begin_tx();

        // If f returns err, no commit will be applied into backend
        // storage
        let ret = execute(&tx)?;

        // Apply changes in atomic way
        tx.commit()?;

        Ok(ret)
    }

    fn close(&mut self) {}
}

pub struct DBTransaction<'db, DB: DBAccess> {
    inner: rocksdb_lib::Transaction<'db, DB>,
    /// cumulative size of transaction footprint
    cumulative_inner_size: RefCell<usize>,

    // TODO: pack all column families into a single array
    // Candidates column family
    candidates_cf: &'db ColumnFamily,
    candidates_height_cf: &'db ColumnFamily,

    // Ledger column families
    ledger_cf: &'db ColumnFamily,
    ledger_txs_cf: &'db ColumnFamily,
    ledger_height_cf: &'db ColumnFamily,

    // Mempool column families
    mempool_cf: &'db ColumnFamily,
    nullifiers_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    metadata_cf: &'db ColumnFamily,

    snapshot: SnapshotWithThreadMode<'db, DB>,
}

impl<'db, DB: DBAccess> Ledger for DBTransaction<'db, DB> {
    fn store_block(
        &self,
        header: &ledger::Header,
        txs: &[SpentTransaction],
        label: Label,
    ) -> Result<usize> {
        // COLUMN FAMILY: CF_LEDGER_HEADER
        // It consists of one record per block - Header record
        // It also includes single record to store metadata - Register record
        {
            let cf = self.ledger_cf;

            let mut buf = vec![];
            HeaderRecord {
                header: header.clone(),
                transactions_ids: txs
                    .iter()
                    .map(|t| t.inner.hash())
                    .collect::<Vec<[u8; 32]>>(),
            }
            .write(&mut buf)?;

            self.put_cf(cf, header.hash, buf)?;
        }

        // Update metadata values
        self.op_write(MD_HASH_KEY, header.hash)?;
        self.op_write(MD_STATE_ROOT_KEY, header.state_hash)?;

        // COLUMN FAMILY: CF_LEDGER_TXS
        {
            let cf = self.ledger_txs_cf;

            // store all block transactions
            for tx in txs {
                let mut d = vec![];
                tx.write(&mut d)?;
                self.put_cf(cf, tx.inner.hash(), d)?;
            }
        }

        // CF: HEIGHT -> (BLOCK_HASH, BLOCK_LABEL)
        let mut buf = vec![];
        buf.write_all(&header.hash[..])?;
        label.write(&mut buf)?;

        self.put_cf(self.ledger_height_cf, header.height.to_le_bytes(), buf)?;

        Ok(self.get_size())
    }

    fn delete_block(&self, b: &ledger::Block) -> Result<()> {
        self.inner.delete_cf(
            self.ledger_height_cf,
            b.header().height.to_le_bytes(),
        )?;

        for tx in b.txs() {
            self.inner.delete_cf(self.ledger_txs_cf, tx.hash())?;
        }

        self.inner.delete_cf(self.ledger_cf, b.header().hash)?;

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
                    let buf = buf?.unwrap();
                    let tx =
                        ledger::SpentTransaction::read(&mut &buf.to_vec()[..])?;
                    txs.push(tx.inner);
                }

                Ok(Some(
                    ledger::Block::new(record.header, txs)
                        .expect("block should be valid"),
                ))
            }
            None => Ok(None),
        }
    }

    fn fetch_block_header(
        &self,
        hash: &[u8],
    ) -> Result<Option<(ledger::Header, Vec<[u8; 32]>)>> {
        match self.snapshot.get_cf(self.ledger_cf, hash)? {
            Some(blob) => {
                let record = HeaderRecord::read(&mut &blob[..])?;
                Ok(Some((record.header, record.transactions_ids)))
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
                const LEN: usize = 32;
                let mut hash = [0u8; LEN];
                hash.copy_from_slice(&h.as_slice()[0..LEN]);
                hash
            }))
    }

    fn get_ledger_tx_by_hash(
        &self,
        tx_hash: &[u8],
    ) -> Result<Option<ledger::SpentTransaction>> {
        let tx = self
            .snapshot
            .get_cf(self.ledger_txs_cf, tx_hash)?
            .map(|blob| ledger::SpentTransaction::read(&mut &blob[..]))
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

    fn fetch_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<ledger::Block>> {
        let hash = self.fetch_block_hash_by_height(height)?;
        let block = match hash {
            Some(hash) => self.fetch_block(&hash)?,
            None => None,
        };
        Ok(block)
    }

    fn fetch_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Label>> {
        const LEN: usize = 32 + 1;
        Ok(self
            .snapshot
            .get_cf(self.ledger_height_cf, height.to_le_bytes())?
            .filter(|v| v.len() == LEN)
            .map(|h| Label::from(h[LEN - 1])))
    }
}

/// Implementation of the `Candidate` trait for `DBTransaction<'db, DB>`.
impl<'db, DB: DBAccess> Candidate for DBTransaction<'db, DB> {
    /// Stores a candidate block in the database.
    ///
    /// # Arguments
    ///
    /// * `b` - The block to store.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the block is successfully stored, or an error if the
    /// operation fails.
    fn store_candidate_block(&self, b: ledger::Block) -> Result<()> {
        let mut serialized = vec![];
        b.write(&mut serialized)?;

        self.inner
            .put_cf(self.candidates_cf, b.header().hash, serialized)?;

        let key = serialize_key(b.header().height, b.header().hash)?;
        self.inner
            .put_cf(self.candidates_height_cf, key, b.header().hash)?;

        Ok(())
    }

    /// Fetches a candidate block from the database.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash of the block to fetch.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(block))` if the block is found, `Ok(None)` if the block
    /// is not found, or an error if the operation fails.
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

    /// Deletes candidate-related items from the database based on a closure.
    ///
    /// # Arguments
    ///
    /// * `closure` - If the closure returns `true`, the block will be deleted.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the deletion is successful, or an error if the
    /// operation fails.
    fn delete<F>(&self, closure: F) -> Result<()>
    where
        F: FnOnce(u64) -> bool + std::marker::Copy,
    {
        let iter = self
            .inner
            .iterator_cf(self.candidates_height_cf, IteratorMode::Start);

        for (key, hash) in iter.map(Result::unwrap) {
            let (height, _) = deserialize_key(&mut &key.to_vec()[..])?;
            if closure(height) {
                self.inner.delete_cf(self.candidates_cf, hash)?;
                self.inner.delete_cf(self.candidates_height_cf, key)?;
            }
        }

        Ok(())
    }

    fn count(&self) -> usize {
        let iter = self
            .inner
            .iterator_cf(self.candidates_height_cf, IteratorMode::Start);

        iter.count()
    }

    /// Deletes all items from the `CF_CANDIDATES` column family.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the deletion is successful, or an error if the
    /// operation fails.
    fn clear_candidates(&self) -> Result<()> {
        self.delete(|_| true)
    }
}

impl<'db, DB: DBAccess> Persist for DBTransaction<'db, DB> {
    /// Deletes all items from both CF_LEDGER and CF_CANDIDATES column families
    fn clear_database(&self) -> Result<()> {
        // Create an iterator over the column family CF_LEDGER
        let iter = self.inner.iterator_cf(self.ledger_cf, IteratorMode::Start);

        // Iterate through the CF_LEDGER column family and delete all items
        for (key, _) in iter.map(Result::unwrap) {
            self.inner.delete_cf(self.ledger_cf, key)?;
        }

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
        let mut tx_data = vec![];
        tx.write(&mut tx_data)?;

        let hash = tx.hash();
        self.put_cf(self.mempool_cf, hash, tx_data)?;

        // Add Secondary indexes //
        // Nullifiers
        for n in tx.inner.nullifiers().iter() {
            let key = n.to_bytes();
            self.put_cf(self.nullifiers_cf, key, hash)?;
        }

        // Map Fee_Hash to Null to facilitate sort-by-fee
        self.put_cf(
            self.fees_cf,
            serialize_key(tx.gas_price(), hash)?,
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
            for n in tx.inner.nullifiers().iter() {
                let key = n.to_bytes();
                self.inner.delete_cf(self.nullifiers_cf, key)?;
            }

            // Delete Fee_Hash
            self.inner.delete_cf(
                self.fees_cf,
                serialize_key(tx.gas_price(), hash)?,
            )?;

            return Ok(true);
        }

        Ok(false)
    }

    fn get_txs_by_nullifiers(&self, n: &[[u8; 32]]) -> HashSet<[u8; 32]> {
        n.iter()
            .filter_map(|n| match self.snapshot.get_cf(self.nullifiers_cf, n) {
                Ok(Some(tx_hash)) => tx_hash.try_into().ok(),
                _ => None,
            })
            .collect()
    }

    fn get_txs_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = ledger::Transaction> + '_>> {
        let iter = MemPoolIterator::new(&self.inner, self.fees_cf, self);

        Ok(Box::new(iter))
    }

    fn get_txs_hashes_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>> {
        let iter = MemPoolFeeIterator::new(&self.inner, self.fees_cf);

        Ok(Box::new(iter))
    }

    fn get_txs_hashes(&self) -> Result<Vec<[u8; 32]>> {
        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_last();

        let mut txs_list = vec![];

        // Iterate all keys from the end in reverse lexicographic order
        while iter.valid() {
            if let Some(key) = iter.key() {
                let (_, tx_hash) = deserialize_key(&mut &key.to_vec()[..])?;

                txs_list.push(tx_hash);
            }

            iter.prev();
        }

        Ok(txs_list)
    }
}

pub struct MemPoolIterator<'db, DB: DBAccess, M: Mempool> {
    iter: MemPoolFeeIterator<'db, DB>,
    mempool: &'db M,
}

impl<'db, DB: DBAccess, M: Mempool> MemPoolIterator<'db, DB, M> {
    fn new(
        db: &'db Transaction<DB>,
        fees_cf: &ColumnFamily,
        mempool: &'db M,
    ) -> Self {
        let iter = MemPoolFeeIterator::new(db, fees_cf);
        MemPoolIterator { iter, mempool }
    }
}

impl<DB: DBAccess, M: Mempool> Iterator for MemPoolIterator<'_, DB, M> {
    type Item = ledger::Transaction;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(|(_, tx_hash)| {
            self.mempool.get_tx(tx_hash).ok().flatten()
        })
    }
}

pub struct MemPoolFeeIterator<'db, DB: DBAccess> {
    iter: DBRawIteratorWithThreadMode<'db, rocksdb_lib::Transaction<'db, DB>>,
}

impl<'db, DB: DBAccess> MemPoolFeeIterator<'db, DB> {
    fn new(db: &'db Transaction<DB>, fees_cf: &ColumnFamily) -> Self {
        let mut iter = db.raw_iterator_cf(fees_cf);
        iter.seek_to_last();
        MemPoolFeeIterator { iter }
    }
}

impl<DB: DBAccess> Iterator for MemPoolFeeIterator<'_, DB> {
    type Item = (u64, [u8; 32]);
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.valid() {
            true => {
                if let Some(key) = self.iter.key() {
                    let (gas_price, hash) =
                        deserialize_key(&mut &key.to_vec()[..]).ok()?;
                    self.iter.prev();
                    Some((gas_price, hash))
                } else {
                    None
                }
            }
            false => None,
        }
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
                writeln!(f, "ledger_block [{}]: {:#?}", b.header().height, b)
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
                        b.header().height,
                        b
                    )
                } else {
                    Ok(())
                }
            });

        results
    }
}

impl<'db, DB: DBAccess> Metadata for DBTransaction<'db, DB> {
    fn op_write<T: AsRef<[u8]>>(&self, key: &[u8], value: T) -> Result<()> {
        self.put_cf(self.metadata_cf, key, value)?;
        Ok(())
    }

    fn op_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.inner.get_cf(self.metadata_cf, key).map_err(Into::into)
    }
}

impl<'db, DB: DBAccess> DBTransaction<'db, DB> {
    /// A thin wrapper around inner.put_cf that calculates a db transaction
    /// disk footprint
    fn put_cf<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        value: V,
    ) -> Result<()> {
        let kv_size = key.as_ref().len() + value.as_ref().len();
        self.inner.put_cf(cf, key, value)?;
        *self.cumulative_inner_size.borrow_mut() += kv_size;
        Ok(())
    }

    pub fn get_size(&self) -> usize {
        *self.cumulative_inner_size.borrow()
    }
}

fn serialize_key(value: u64, hash: [u8; 32]) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &value.to_be_bytes())?;
    std::io::Write::write_all(&mut w, &hash)?;
    Ok(w)
}

fn deserialize_key<R: Read>(r: &mut R) -> Result<(u64, [u8; 32])> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let value = u64::from_be_bytes(buf);
    let mut hash = [0u8; 32];
    r.read_exact(&mut hash[..])?;

    Ok((value, hash))
}

struct HeaderRecord {
    header: ledger::Header,
    transactions_ids: Vec<[u8; 32]>,
}

impl node_data::Serializable for HeaderRecord {
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
        let len = Self::read_u32_le(r)?;

        // Read transactions hashes
        let mut transactions_ids = vec![];
        for _ in 0..len {
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

    use fake::{Fake, Faker};
    use node_data::ledger::Transaction;

    #[test]
    fn test_store_block() {
        TestWrapper::new("test_store_block").run(|path| {
            let db: Backend = Backend::create_or_open(path);

            let b: ledger::Block = Faker.fake();
            assert!(!b.txs().is_empty());

            let hash = b.header().hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        Label::Final,
                    )?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                // Assert block header is fully fetched from ledger
                let db_blk = txn
                    .fetch_block(&hash)
                    .expect("Block to be fetched")
                    .expect("Block to exist");
                assert_eq!(db_blk.header().hash, b.header().hash);

                // Assert all transactions are fully fetched from ledger as
                // well.
                for pos in 0..b.txs().len() {
                    assert_eq!(db_blk.txs()[pos].hash(), b.txs()[pos].hash());
                }
            });

            assert!(db
                .update(|txn| {
                    txn.clear_database()?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn
                    .fetch_block(&hash)
                    .expect("block to be fetched")
                    .is_none());
            });
        });
    }

    #[test]
    fn test_read_only() {
        TestWrapper::new("test_read_only").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let b: ledger::Block = Faker.fake();
            db.view(|txn| {
                txn.store_block(
                    b.header(),
                    &to_spent_txs(b.txs()),
                    Label::Final,
                )
                .expect("block to be stored");
            });
            db.view(|txn| {
                assert!(txn
                    .fetch_block(&b.header().hash)
                    .expect("block to be fetched")
                    .is_none());
            });
        });
    }

    #[test]
    fn test_transaction_isolation() {
        TestWrapper::new("test_transaction_isolation").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let mut b: ledger::Block = Faker.fake();
            let hash = b.header().hash;

            db.view(|txn| {
                // Simulate a concurrent update is committed during read-only
                // transaction
                assert!(db
                    .update(|txn| {
                        txn.store_block(
                            b.header(),
                            &to_spent_txs(b.txs()),
                            Label::Final,
                        )
                        .unwrap();

                        // No need to support Read-Your-Own-Writes
                        assert!(txn.fetch_block(&hash)?.is_none());
                        Ok(())
                    })
                    .is_ok());

                // Asserts that the read-only/view transaction runs in isolation
                assert!(txn
                    .fetch_block(&hash)
                    .expect("block to be fetched")
                    .is_none());
            });

            // Asserts that update was done
            db.view(|txn| {
                assert_blocks_eq(
                    &mut txn
                        .fetch_block(&hash)
                        .expect("block to be fetched")
                        .unwrap(),
                    &mut b,
                );
            });
        });
    }

    fn assert_blocks_eq(a: &mut ledger::Block, b: &mut ledger::Block) {
        assert!(a.header().hash != [0u8; 32]);
        assert!(a.header().hash.eq(&b.header().hash));
    }

    #[test]
    fn test_add_mempool_tx() {
        TestWrapper::new("test_add_tx").run(|path| {
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
            });

            // Delete a contract call
            db.update(|txn| {
                assert!(txn.delete_tx(t.hash()).expect("valid tx"));
                Ok(())
            })
            .unwrap();
        });
    }

    #[test]
    fn test_mempool_txs_sorted_by_fee() {
        TestWrapper::new("test_mempool_txs_sorted_by_fee").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            // Populate mempool with N contract calls
            let _rng = rand::thread_rng();
            db.update(|txn| {
                for _i in 0..10u32 {
                    let t: ledger::Transaction = Faker.fake();
                    txn.add_tx(&t)?;
                }
                Ok(())
            })
            .unwrap();

            db.view(|txn| {
                let txs =
                    txn.get_txs_sorted_by_fee().expect("iter should return");

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
    fn test_max_gas_limit() {
        TestWrapper::new("test_block_size_limit").run(|path| {
            let db: Backend = Backend::create_or_open(path);

            db.update(|txn| {
                for i in 0..10u32 {
                    let t = ledger::faker::gen_dummy_tx(i as u64);
                    txn.add_tx(&t)?;
                }
                Ok(())
            })
            .unwrap();

            let total_gas_price: u64 = 9 + 8 + 7 + 6 + 5 + 4 + 3 + 2 + 1;
            db.view(|txn| {
                let txs = txn
                    .get_txs_sorted_by_fee()
                    .expect("should return all txs")
                    .map(|t| t.gas_price())
                    .sum::<u64>();

                assert_eq!(txs, total_gas_price);
            });
        });
    }

    fn to_spent_txs(txs: &Vec<Transaction>) -> Vec<SpentTransaction> {
        txs.iter()
            .map(|t| SpentTransaction {
                inner: t.clone(),
                block_height: 0,
                gas_spent: 0,
                err: None,
            })
            .collect()
    }

    #[test]
    fn test_get_ledger_tx_by_hash() {
        TestWrapper::new("test_get_ledger_tx_by_hash").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let b: ledger::Block = Faker.fake();
            assert!(!b.txs().is_empty());

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        Label::Final,
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert all transactions of the accepted (stored) block are
            // accessible by hash.
            db.view(|v| {
                for t in b.txs().iter() {
                    assert!(v
                        .get_ledger_tx_by_hash(&t.hash())
                        .expect("should not return error")
                        .expect("should find a transaction")
                        .inner
                        .eq(t));
                }
            });
        });
    }

    #[test]
    fn test_fetch_block_hash_by_height() {
        TestWrapper::new("test_fetch_block_hash_by_height").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let b: ledger::Block = Faker.fake();

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        Label::Attested,
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(v
                    .fetch_block_hash_by_height(b.header().height)
                    .expect("should not return error")
                    .expect("should find a block")
                    .eq(&b.header().hash));
            });
        });
    }

    #[test]
    fn test_fetch_block_label_by_height() {
        TestWrapper::new("test_fetch_block_hash_by_height").run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let b: ledger::Block = Faker.fake();

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        Label::Attested,
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(v
                    .fetch_block_label_by_height(b.header().height)
                    .expect("should not return error")
                    .expect("should find a block")
                    .eq(&Label::Attested));
            });
        });
    }

    #[test]
    /// Ensures delete_block fn removes all keys of a single block
    fn test_delete_block() {
        let t = TestWrapper::new("test_fetch_block_hash_by_height");
        t.run(|path| {
            let db: Backend = Backend::create_or_open(path);
            let b: ledger::Block = Faker.fake();

            assert!(db
                .update(|ut| {
                    ut.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        Label::Final,
                    )?;
                    Ok(())
                })
                .is_ok());

            assert!(db
                .update(|ut| {
                    ut.delete_block(&b)?;
                    Ok(())
                })
                .is_ok());
        });

        let path = t.get_path();
        let opts = Options::default();

        let vec = rocksdb_lib::DB::list_cf(&opts, &path).unwrap();
        assert!(!vec.is_empty());

        // Ensure no block fields leak after its deletion
        let db = rocksdb_lib::DB::open_cf(&opts, &path, vec.clone()).unwrap();
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

    struct TestWrapper(tempdir::TempDir);

    impl TestWrapper {
        fn new(path: &'static str) -> Self {
            Self(
                tempdir::TempDir::new(path)
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
