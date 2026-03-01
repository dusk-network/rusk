// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
/// Implementation of the `Candidate` trait for `DBTransaction<'db, DB>`.
impl<DB: DBAccess> ConsensusStorage for DBTransaction<'_, DB> {
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

impl<DB: DBAccess> Mempool for DBTransaction<'_, DB> {
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
    ) -> Box<dyn Iterator<Item = Transaction> + '_> {
        let iter = MemPoolIterator::new(&self.inner, self.fees_cf, self);

        Box::new(iter)
    }

    fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Box<dyn Iterator<Item = (u64, [u8; 32])> + '_> {
        let iter = MemPoolFeeIterator::new(&self.inner, self.fees_cf, true);

        Box::new(iter)
    }

    fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Box<dyn Iterator<Item = (u64, [u8; 32])> + '_> {
        let iter = MemPoolFeeIterator::new(&self.inner, self.fees_cf, false);

        Box::new(iter)
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
