// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

enum FinalizeOp {
    Commit,
    Rollback,
}

impl<DB: DBAccess> Persist for DBTransaction<'_, DB> {
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
        self.finalize(FinalizeOp::Commit)
    }

    fn rollback(self) -> Result<()> {
        self.finalize(FinalizeOp::Rollback)
    }
}

impl<DB: DBAccess> std::fmt::Debug for DBTransaction<'_, DB> {
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

impl<DB: DBAccess> DBTransaction<'_, DB> {
    fn finalize(self, op: FinalizeOp) -> Result<()> {
        match op {
            FinalizeOp::Commit => self
                .inner
                .commit()
                .map_err(error::RocksDbError::commit_failed)?,
            FinalizeOp::Rollback => self
                .inner
                .rollback()
                .map_err(error::RocksDbError::rollback_failed)?,
        }
        Ok(())
    }

    /// A thin wrapper around inner.put_cf that calculates a db transaction
    /// disk footprint
    pub(super) fn put_cf<K: AsRef<[u8]>, V: AsRef<[u8]>>(
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

    pub(super) fn get_size(&self) -> usize {
        *self.cumulative_inner_size.borrow()
    }
}
