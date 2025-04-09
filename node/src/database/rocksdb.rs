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
use node_data::ledger::{
    Block, Fault, Header, Label, SpendingId, SpentTransaction, Transaction,
};
use node_data::message::{payload, ConsensusHeader};
use node_data::Serializable;
use rocksdb::{
    AsColumnFamilyRef, BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor,
    DBAccess, DBRawIteratorWithThreadMode, IteratorMode, LogLevel,
    OptimisticTransactionDB, OptimisticTransactionOptions, Options,
    WriteOptions,
};
use tracing::info;

use super::{
    ConsensusStorage, DatabaseOptions, Ledger, LightBlock, Metadata, Persist,
    DB,
};
use crate::database::Mempool;

const CF_LEDGER_HEADER: &str = "cf_ledger_header";
const CF_LEDGER_TXS: &str = "cf_ledger_txs";
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
                    path,
                    cfs,
                )
                .expect("should be a valid database in {path}"),
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

    // Mempool column families
    mempool_cf: &'db ColumnFamily,
    spending_id_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    metadata_cf: &'db ColumnFamily,
}

impl<'db, DB: DBAccess> Ledger for DBTransaction<'db, DB> {
    fn store_block(
        &mut self,
        header: &Header,
        txs: &[SpentTransaction],
        faults: &[Fault],
        label: Label,
    ) -> Result<usize> {
        // COLUMN FAMILY: CF_LEDGER_HEADER
        // It consists of one record per block - Header record
        // It also includes single record to store metadata - Register record
        {
            let cf = self.ledger_cf;

            let mut buf = vec![];
            LightBlock {
                header: header.clone(),
                transactions_ids: txs.iter().map(|t| t.inner.id()).collect(),
                faults_ids: faults.iter().map(|f| f.id()).collect(),
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
                self.put_cf(cf, tx.inner.id(), d)?;
            }
        }

        // COLUMN FAMILY: CF_LEDGER_FAULTS
        {
            let cf = self.ledger_faults_cf;

            // store all block faults
            for f in faults {
                let mut d = vec![];
                f.write(&mut d)?;
                self.put_cf(cf, f.id(), d)?;
            }
        }
        self.store_block_label(header.height, &header.hash, label)?;

        Ok(self.get_size())
    }

    fn faults_by_block(&self, start_height: u64) -> Result<Vec<Fault>> {
        let mut faults = vec![];
        let mut hash = self
            .op_read(MD_HASH_KEY)?
            .ok_or(anyhow::anyhow!("Cannot read tip"))?;

        loop {
            let block = self.light_block(&hash)?.ok_or(anyhow::anyhow!(
                "Cannot read block {}",
                hex::encode(&hash)
            ))?;

            let block_height = block.header.height;

            if block_height >= start_height {
                hash = block.header.prev_block_hash.to_vec();
                faults.extend(self.faults(&block.faults_ids)?);
            } else {
                break;
            }

            if block_height == 0 {
                break;
            }
        }
        Ok(faults)
    }

    fn store_block_label(
        &mut self,
        height: u64,
        hash: &[u8; 32],
        label: Label,
    ) -> Result<()> {
        // CF: HEIGHT -> (BLOCK_HASH, BLOCK_LABEL)
        let mut buf = vec![];
        buf.write_all(hash)?;
        label.write(&mut buf)?;

        self.put_cf(self.ledger_height_cf, height.to_le_bytes(), buf)?;
        Ok(())
    }

    fn delete_block(&mut self, b: &Block) -> Result<()> {
        self.inner.delete_cf(
            self.ledger_height_cf,
            b.header().height.to_le_bytes(),
        )?;

        for tx in b.txs() {
            self.inner.delete_cf(self.ledger_txs_cf, tx.id())?;
        }
        for f in b.faults() {
            self.inner.delete_cf(self.ledger_faults_cf, f.id())?;
        }

        self.inner.delete_cf(self.ledger_cf, b.header().hash)?;

        Ok(())
    }

    fn block_exists(&self, hash: &[u8]) -> Result<bool> {
        Ok(self.inner.get_cf(self.ledger_cf, hash)?.is_some())
    }

    fn faults(&self, faults_ids: &[[u8; 32]]) -> Result<Vec<Fault>> {
        if faults_ids.is_empty() {
            return Ok(vec![]);
        }
        let ids = faults_ids
            .iter()
            .map(|id| (self.ledger_faults_cf, id))
            .collect::<Vec<_>>();

        // Retrieve all faults ID with single call
        let faults_buffer = self.inner.multi_get_cf(ids);

        let mut faults = vec![];
        for buf in faults_buffer {
            let buf = buf?.unwrap();
            let fault = Fault::read(&mut &buf[..])?;
            faults.push(fault);
        }

        Ok(faults)
    }

    fn block(&self, hash: &[u8]) -> Result<Option<Block>> {
        match self.inner.get_cf(self.ledger_cf, hash)? {
            Some(blob) => {
                let record = LightBlock::read(&mut &blob[..])?;

                // Retrieve all transactions buffers with single call
                let txs_buffers = self.inner.multi_get_cf(
                    record
                        .transactions_ids
                        .iter()
                        .map(|id| (self.ledger_txs_cf, id))
                        .collect::<Vec<(&ColumnFamily, &[u8; 32])>>(),
                );

                let mut txs = vec![];
                for buf in txs_buffers {
                    let buf = buf?.unwrap();
                    let tx = SpentTransaction::read(&mut &buf[..])?;
                    txs.push(tx.inner);
                }

                // Retrieve all faults ID with single call
                let faults_buffer = self.inner.multi_get_cf(
                    record
                        .faults_ids
                        .iter()
                        .map(|id| (self.ledger_faults_cf, id))
                        .collect::<Vec<(&ColumnFamily, &[u8; 32])>>(),
                );
                let mut faults = vec![];
                for buf in faults_buffer {
                    let buf = buf?.unwrap();
                    let fault = Fault::read(&mut &buf[..])?;
                    faults.push(fault);
                }

                Ok(Some(
                    Block::new(record.header, txs, faults)
                        .expect("block should be valid"),
                ))
            }
            None => Ok(None),
        }
    }

    fn light_block(&self, hash: &[u8]) -> Result<Option<LightBlock>> {
        match self.inner.get_cf(self.ledger_cf, hash)? {
            Some(blob) => {
                let record = LightBlock::read(&mut &blob[..])?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    fn block_header(&self, hash: &[u8]) -> Result<Option<Header>> {
        match self.inner.get_cf(self.ledger_cf, hash)? {
            Some(blob) => {
                let record = Header::read(&mut &blob[..])?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    fn block_hash_by_height(&self, height: u64) -> Result<Option<[u8; 32]>> {
        Ok(self
            .inner
            .get_cf(self.ledger_height_cf, height.to_le_bytes())?
            .map(|h| {
                const LEN: usize = 32;
                let mut hash = [0u8; LEN];
                hash.copy_from_slice(&h.as_slice()[0..LEN]);
                hash
            }))
    }

    fn ledger_tx(&self, tx_id: &[u8]) -> Result<Option<SpentTransaction>> {
        let tx = self
            .inner
            .get_cf(self.ledger_txs_cf, tx_id)?
            .map(|blob| SpentTransaction::read(&mut &blob[..]))
            .transpose()?;

        Ok(tx)
    }

    /// Returns a list of transactions from the ledger
    ///
    /// This function expects a list of transaction IDs that are in the ledger.
    ///
    /// It will return an error if any of the transaction IDs are not found in
    /// the ledger.
    fn ledger_txs(
        &self,
        tx_ids: Vec<&[u8; 32]>,
    ) -> Result<Vec<SpentTransaction>> {
        let cf = self.ledger_txs_cf;

        let ids = tx_ids.into_iter().map(|id| (cf, id)).collect::<Vec<_>>();

        let multi_get_results = self.inner.multi_get_cf(ids);

        let mut spent_transactions =
            Vec::with_capacity(multi_get_results.len());
        for result in multi_get_results.into_iter() {
            let opt_blob = result.map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?;

            let Some(blob) = opt_blob else {
                return Err(anyhow::anyhow!(
                    "At least one Transaction ID was not found"
                ));
            };

            let stx = SpentTransaction::read(&mut &blob[..])?;

            spent_transactions.push(stx);
        }

        Ok(spent_transactions)
    }

    fn ledger_tx_count(&self) -> usize {
        self.inner
            .iterator_cf(self.ledger_txs_cf, IteratorMode::Start)
            .count()
    }

    /// Returns true if the transaction exists in the
    /// ledger
    ///
    /// This is a convenience method that checks if a transaction exists in the
    /// ledger without unmarshalling the transaction
    fn ledger_tx_exists(&self, tx_id: &[u8]) -> Result<bool> {
        Ok(self.inner.get_cf(self.ledger_txs_cf, tx_id)?.is_some())
    }

    fn block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let hash = self.block_hash_by_height(height)?;
        let block = match hash {
            Some(hash) => self.block(&hash)?,
            None => None,
        };
        Ok(block)
    }

    fn block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<([u8; 32], Label)>> {
        const HASH_LEN: usize = 32;
        Ok(self
            .inner
            .get_cf(self.ledger_height_cf, height.to_le_bytes())?
            .map(|h| {
                let mut hash = [0u8; HASH_LEN];
                hash.copy_from_slice(&h.as_slice()[0..HASH_LEN]);

                let label_buff = h[HASH_LEN..].to_vec();
                Label::read(&mut &label_buff[..]).map(|label| (hash, label))
            })
            .transpose()?)
    }
}

/// Implementation of the `Candidate` trait for `DBTransaction<'db, DB>`.
impl<'db, DB: DBAccess> ConsensusStorage for DBTransaction<'db, DB> {
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
    fn store_candidate(&mut self, b: Block) -> Result<()> {
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
    fn candidate(&self, hash: &[u8]) -> Result<Option<Block>> {
        if let Some(blob) = self.inner.get_cf(self.candidates_cf, hash)? {
            let b = Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }

    fn candidate_by_iteration(
        &self,
        consensus_header: &ConsensusHeader,
    ) -> Result<Option<Block>> {
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, IteratorMode::Start);

        for (_, blob) in iter.map(Result::unwrap) {
            let b = Block::read(&mut &blob[..])?;

            let header = b.header();
            if header.prev_block_hash == consensus_header.prev_block_hash
                && header.iteration == consensus_header.iteration
            {
                return Ok(Some(b));
            }
        }

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
    fn delete_candidate<F>(&mut self, closure: F) -> Result<()>
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

    fn count_candidates(&self) -> usize {
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
    fn clear_candidates(&mut self) -> Result<()> {
        self.delete_candidate(|_| true)
    }

    /// Stores a ValidationResult in the database.
    ///
    /// # Arguments
    ///
    /// * `vr` - The ValidationResult to store.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the ValidationResult is successfully stored, or an
    /// error if the operation fails.
    fn store_validation_result(
        &mut self,
        consensus_header: &ConsensusHeader,
        validation_result: &payload::ValidationResult,
    ) -> Result<()> {
        let mut serialized = vec![];
        validation_result.write(&mut serialized)?;

        let key = serialize_iter_key(consensus_header)?;
        self.inner
            .put_cf(self.validation_results_cf, key, serialized)?;

        Ok(())
    }

    /// Fetches a ValidationResult from the database.
    ///
    /// # Arguments
    ///
    /// * `consensus_header` - The ConsensusHeader of the ValidationResult.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(ValidationResult))` if the ValidationResult is found,
    /// `Ok(None)` if the ValidationResult is not found, or an error if the
    /// operation fails.
    fn validation_result(
        &self,
        consensus_header: &ConsensusHeader,
    ) -> Result<Option<payload::ValidationResult>> {
        let key = serialize_iter_key(consensus_header)?;
        if let Some(blob) =
            self.inner.get_cf(self.validation_results_cf, key)?
        {
            let validation_result =
                payload::ValidationResult::read(&mut &blob[..])?;
            return Ok(Some(validation_result));
        }

        // ValidationResult not found
        Ok(None)
    }

    /// Deletes ValidationResult items from the database based on a closure.
    ///
    /// # Arguments
    ///
    /// * `closure` - If the closure returns `true`, the ValidationResult will
    ///   be deleted.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the deletion is successful, or an error if the
    /// operation fails.
    fn delete_validation_results<F>(&mut self, closure: F) -> Result<()>
    where
        F: FnOnce([u8; 32]) -> bool + std::marker::Copy,
    {
        let iter = self
            .inner
            .iterator_cf(self.validation_results_cf, IteratorMode::Start);

        for (key, _) in iter.map(Result::unwrap) {
            let (prev_block_hash, _) =
                deserialize_iter_key(&mut &key.to_vec()[..])?;
            if closure(prev_block_hash) {
                self.inner.delete_cf(self.validation_results_cf, key)?;
            }
        }

        Ok(())
    }

    fn count_validation_results(&self) -> usize {
        let iter = self
            .inner
            .iterator_cf(self.validation_results_cf, IteratorMode::Start);

        iter.count()
    }

    /// Deletes all items from the `CF_VALIDATION_RESULTS` column family.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the deletion is successful, or an error if the
    /// operation fails.
    fn clear_validation_results(&mut self) -> Result<()> {
        self.delete_validation_results(|_| true)
    }
}

impl<'db, DB: DBAccess> Persist for DBTransaction<'db, DB> {
    /// Deletes all items from both CF_LEDGER and CF_CANDIDATES column families
    fn clear_database(&mut self) -> Result<()> {
        // Create an iterator over the column family CF_LEDGER
        let iter = self.inner.iterator_cf(self.ledger_cf, IteratorMode::Start);

        // Iterate through the CF_LEDGER column family and delete all items
        for (key, _) in iter.map(Result::unwrap) {
            self.inner.delete_cf(self.ledger_cf, key)?;
        }

        self.clear_candidates()?;
        self.clear_validation_results()?;
        Ok(())
    }

    fn commit(self) -> Result<()> {
        if let Err(e) = self.inner.commit() {
            return Err(anyhow::Error::new(e).context("failed to commit"));
        }

        Ok(())
    }

    fn rollback(self) -> Result<()> {
        if let Err(e) = self.inner.rollback() {
            return Err(anyhow::Error::new(e).context("failed to rollback"));
        }

        Ok(())
    }
}

impl<'db, DB: DBAccess> Mempool for DBTransaction<'db, DB> {
    fn store_mempool_tx(
        &mut self,
        tx: &Transaction,
        timestamp: u64,
    ) -> Result<()> {
        // Map Hash to serialized transaction
        let mut tx_data = vec![];
        tx.write(&mut tx_data)?;

        let hash = tx.id();
        self.put_cf(self.mempool_cf, hash, tx_data)?;

        // Add Secondary indexes //
        // Spending Ids
        for n in tx.to_spend_ids() {
            let key = n.to_bytes();
            self.put_cf(self.spending_id_cf, key, hash)?;
        }

        let timestamp = timestamp.to_be_bytes();

        // Map Fee_Hash to Timestamp
        // Key pair is used to facilitate sort-by-fee
        // Also, the timestamp is used to remove expired transactions
        self.put_cf(
            self.fees_cf,
            serialize_key(tx.gas_price(), hash)?,
            timestamp,
        )?;

        Ok(())
    }

    fn mempool_tx(&self, hash: [u8; 32]) -> Result<Option<Transaction>> {
        let data = self.inner.get_cf(self.mempool_cf, hash)?;

        match data {
            // None has a meaning key not found
            None => Ok(None),
            Some(blob) => Ok(Some(Transaction::read(&mut &blob.to_vec()[..])?)),
        }
    }

    fn mempool_tx_exists(&self, h: [u8; 32]) -> Result<bool> {
        Ok(self.inner.get_cf(self.mempool_cf, h)?.is_some())
    }

    fn delete_mempool_tx(
        &mut self,
        h: [u8; 32],
        cascade: bool,
    ) -> Result<Vec<[u8; 32]>> {
        let mut deleted = vec![];
        let tx = self.mempool_tx(h)?;
        if let Some(tx) = tx {
            let hash = tx.id();

            self.inner.delete_cf(self.mempool_cf, hash)?;

            // Delete Secondary indexes
            // Delete spendingids (nullifiers or nonce)
            for n in tx.to_spend_ids() {
                let key = n.to_bytes();
                self.inner.delete_cf(self.spending_id_cf, key)?;
            }

            // Delete Fee_Hash
            self.inner.delete_cf(
                self.fees_cf,
                serialize_key(tx.gas_price(), hash)?,
            )?;

            deleted.push(h);

            if cascade {
                let mut dependants = vec![];
                // Get the next spending id (aka next nonce tx)
                // retrieve tx_id and delete it
                let mut next_spending_id = tx.next_spending_id();
                while let Some(spending_id) = next_spending_id {
                    next_spending_id = spending_id.next();
                    let next_txs =
                        self.mempool_txs_by_spendable_ids(&[spending_id]);
                    if next_txs.is_empty() {
                        break;
                    }
                    dependants.extend(next_txs);
                }

                // delete all dependants
                for tx_id in dependants {
                    let cascade_deleted =
                        self.delete_mempool_tx(tx_id, false)?;
                    deleted.extend(cascade_deleted);
                }
            }
        }

        Ok(deleted)
    }

    fn mempool_txs_by_spendable_ids(
        &self,
        n: &[SpendingId],
    ) -> HashSet<[u8; 32]> {
        n.iter()
            .filter_map(|n| {
                match self.inner.get_cf(self.spending_id_cf, n.to_bytes()) {
                    Ok(Some(tx_id)) => tx_id.try_into().ok(),
                    _ => None,
                }
            })
            .collect()
    }

    fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Transaction> + '_>> {
        let iter = MemPoolIterator::new(&self.inner, self.fees_cf, self);

        Ok(Box::new(iter))
    }

    fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>> {
        let iter = MemPoolFeeIterator::new(&self.inner, self.fees_cf, true);

        Ok(Box::new(iter))
    }

    fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (u64, [u8; 32])> + '_>> {
        let iter = MemPoolFeeIterator::new(&self.inner, self.fees_cf, false);

        Ok(Box::new(iter))
    }

    /// Get all expired transactions hashes.
    fn mempool_expired_txs(&self, timestamp: u64) -> Result<Vec<[u8; 32]>> {
        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_first();
        let mut txs_list = vec![];

        while iter.valid() {
            if let Some(key) = iter.key() {
                let (_, tx_id) = deserialize_key(&mut &key.to_vec()[..])?;

                let tx_timestamp = u64::from_be_bytes(
                    iter.value()
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "no value",
                            )
                        })?
                        .try_into()
                        .map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid data",
                            )
                        })?,
                );

                if tx_timestamp <= timestamp {
                    txs_list.push(tx_id);
                }
            }

            iter.next();
        }

        Ok(txs_list)
    }

    fn mempool_txs_ids(&self) -> Result<Vec<[u8; 32]>> {
        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_last();

        let mut txs_list = vec![];

        // Iterate all keys from the end in reverse lexicographic order
        while iter.valid() {
            if let Some(key) = iter.key() {
                let (_, tx_id) = deserialize_key(&mut &key.to_vec()[..])?;

                txs_list.push(tx_id);
            }

            iter.prev();
        }

        Ok(txs_list)
    }

    fn mempool_txs_count(&self) -> usize {
        self.inner
            .iterator_cf(self.mempool_cf, IteratorMode::Start)
            .count()
    }
}

pub struct MemPoolIterator<'db, DB: DBAccess, M: Mempool> {
    iter: MemPoolFeeIterator<'db, DB>,
    mempool: &'db M,
}

impl<'db, DB: DBAccess, M: Mempool> MemPoolIterator<'db, DB, M> {
    fn new(
        db: &'db rocksdb::Transaction<DB>,
        fees_cf: &ColumnFamily,
        mempool: &'db M,
    ) -> Self {
        let iter = MemPoolFeeIterator::new(db, fees_cf, true);
        MemPoolIterator { iter, mempool }
    }
}

impl<DB: DBAccess, M: Mempool> Iterator for MemPoolIterator<'_, DB, M> {
    type Item = Transaction;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(|(_, tx_id)| {
            self.mempool.mempool_tx(tx_id).ok().flatten()
        })
    }
}

pub struct MemPoolFeeIterator<'db, DB: DBAccess> {
    iter: DBRawIteratorWithThreadMode<'db, rocksdb::Transaction<'db, DB>>,
    fee_desc: bool,
}

impl<'db, DB: DBAccess> MemPoolFeeIterator<'db, DB> {
    fn new(
        db: &'db rocksdb::Transaction<DB>,
        fees_cf: &ColumnFamily,
        fee_desc: bool,
    ) -> Self {
        let mut iter = db.raw_iterator_cf(fees_cf);
        if fee_desc {
            iter.seek_to_last();
        };
        MemPoolFeeIterator { iter, fee_desc }
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
                    if self.fee_desc {
                        self.iter.prev();
                    } else {
                        self.iter.next();
                    }
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
            if let Ok(Some(blob)) = self.inner.get_cf(self.ledger_cf, &hash[..])
            {
                let b = Block::read(&mut &blob[..]).unwrap_or_default();
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
                    self.inner.get_cf(self.candidates_cf, &hash[..])
                {
                    let b = Block::read(&mut &blob[..]).unwrap_or_default();
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
    fn op_write<T: AsRef<[u8]>>(&mut self, key: &[u8], value: T) -> Result<()> {
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

fn serialize_iter_key(ch: &ConsensusHeader) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &ch.prev_block_hash)?;
    std::io::Write::write_all(&mut w, &[ch.iteration])?;
    Ok(w)
}

fn deserialize_iter_key<R: Read>(r: &mut R) -> Result<([u8; 32], u8)> {
    let mut prev_block_hash = [0u8; 32];
    r.read_exact(&mut prev_block_hash)?;

    let mut iter_byte = [0u8; 1];
    r.read_exact(&mut iter_byte)?;
    let iteration = u8::from_be_bytes(iter_byte);

    Ok((prev_block_hash, iteration))
}

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
    use node_data::ledger;

    use super::*;

    #[test]
    fn test_store_block() {
        TestWrapper::new("test_store_block").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());

            let b: Block = Faker.fake();
            assert!(!b.txs().is_empty());

            let hash = b.header().hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        b.faults(),
                        Label::Final(3),
                    )?;
                    Ok(())
                })
                .is_ok());

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
                    assert_eq!(db_blk.txs()[pos].id(), b.txs()[pos].id());
                }

                // Assert all faults are fully fetched from ledger as
                // well.
                for pos in 0..b.faults().len() {
                    assert_eq!(db_blk.faults()[pos].id(), b.faults()[pos].id());
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
                    .block(&hash)
                    .expect("block to be fetched")
                    .is_none());
            });
        });
    }

    #[test]
    fn test_read_only() {
        TestWrapper::new("test_read_only").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            db.update_dry_run(true, |txn| {
                txn.store_block(
                    b.header(),
                    &to_spent_txs(b.txs()),
                    b.faults(),
                    Label::Final(3),
                )
            })
            .expect("block to be stored");
            db.view(|txn| {
                assert!(txn
                    .block(&b.header().hash)
                    .expect("block to be fetched")
                    .is_none());
            });
        });
    }

    #[test]
    fn test_transaction_isolation() {
        TestWrapper::new("test_transaction_isolation").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let mut b: Block = Faker.fake();
            let hash = b.header().hash;

            db.view(|txn| {
                // Simulate a concurrent update is committed during read-only
                // transaction
                assert!(db
                    .update(|inner| {
                        inner
                            .store_block(
                                b.header(),
                                &to_spent_txs(b.txs()),
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
                    .is_ok());

                // Asserts that the read-only/view transaction get the updated
                // data after the tx is committed
                assert!(txn
                    .block(&hash)
                    .expect("block to be fetched")
                    .is_some());
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
            let t: Transaction = Faker.fake();

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
                    let t: Transaction = Faker.fake();
                    txn.store_mempool_tx(&t, 0)?;
                }
                Ok(())
            })
            .unwrap();

            db.view(|txn| {
                let txs = txn
                    .mempool_txs_sorted_by_fee()
                    .expect("iter should return");

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
                    .expect("should return all txs")
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
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();
            assert!(!b.txs().is_empty());

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        b.faults(),
                        Label::Final(3),
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert all transactions of the accepted (stored) block are
            // accessible by hash.
            db.view(|v| {
                for t in b.txs().iter() {
                    assert!(v
                        .ledger_tx(&t.id())
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
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        b.faults(),
                        Label::Attested(3),
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(v
                    .block_hash_by_height(b.header().height)
                    .expect("should not return error")
                    .expect("should find a block")
                    .eq(&b.header().hash));
            });
        });
    }

    #[test]
    fn test_fetch_block_label_by_height() {
        TestWrapper::new("test_fetch_block_hash_by_height").run(|path| {
            let db = Backend::create_or_open(path, DatabaseOptions::default());
            let b: Block = Faker.fake();

            // Store a block
            assert!(db
                .update(|txn| {
                    txn.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        b.faults(),
                        Label::Attested(3),
                    )?;
                    Ok(())
                })
                .is_ok());

            // Assert block hash is accessible by height.
            db.view(|v| {
                assert!(v
                    .block_label_by_height(b.header().height)
                    .expect("should not return error")
                    .expect("should find a block")
                    .1
                    .eq(&Label::Attested(3)));
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

            assert!(db
                .update(|ut| {
                    ut.store_block(
                        b.header(),
                        &to_spent_txs(b.txs()),
                        b.faults(),
                        Label::Final(3),
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
