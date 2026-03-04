// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
impl<DB: DBAccess> Ledger for DBTransaction<'_, DB> {
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

            let mut stored_blobs = Vec::with_capacity(6);

            // store all block transactions
            for tx in txs {
                let mut d = vec![];

                if tx.inner.inner.blob().is_some() {
                    let mut strip_tx = tx.clone();
                    if let Some(blobs) = strip_tx.inner.inner.strip_blobs() {
                        for (hash, sidecar) in blobs.into_iter() {
                            let sidecar_bytes = sidecar.to_var_bytes();
                            self.store_blob_data(&hash, sidecar_bytes)?;
                            stored_blobs.push(hash);
                        }
                    }
                    strip_tx.write(&mut d)?;
                } else {
                    tx.write(&mut d)?;
                }
                self.put_cf(cf, tx.inner.id(), d)?;
            }

            if !stored_blobs.is_empty() {
                // Store all blobs hashes in the ledger
                self.store_blobs_height(header.height, &stored_blobs)?;
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
            .ok_or(error::RocksDbError::CannotReadTip)?;

        loop {
            let block = self
                .light_block(&hash)?
                .ok_or_else(|| error::RocksDbError::cannot_read_block(&hash))?;

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

        self.delete_blobs_by_height(b.header().height)?;
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

    fn latest_block(&self) -> Result<LightBlock> {
        let tip_hash = self
            .op_read(MD_HASH_KEY)?
            .ok_or(error::RocksDbError::TipMetadataMissing)?;

        let tip_block = self
            .light_block(&tip_hash)?
            .ok_or(error::RocksDbError::TipBlockMissing)?;
        Ok(tip_block)
    }

    fn blob_data_by_hash(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        Ok(self.inner.get_cf(self.ledger_blobs_cf, hash)?)
    }

    fn store_blob_data(&self, hash: &[u8; 32], data: Vec<u8>) -> Result<()> {
        self.inner.put_cf(self.ledger_blobs_cf, hash, data)?;
        Ok(())
    }
    fn store_blobs_height(
        &self,
        block_height: u64,
        blob_hashes: &[[u8; 32]],
    ) -> Result<()> {
        if blob_hashes.is_empty() {
            return Ok(());
        }
        let blob_hashes_bytes: Vec<_> =
            blob_hashes.iter().flat_map(|hash| hash.to_vec()).collect();
        self.inner.put_cf(
            self.ledger_blobs_height_cf,
            block_height.to_be_bytes(),
            blob_hashes_bytes,
        )?;
        Ok(())
    }

    fn delete_blobs_by_height(&self, block_height: u64) -> Result<()> {
        let blobs_to_delete = self.blobs_by_height(block_height)?;
        if let Some(blob_hashes) = blobs_to_delete {
            for hash in blob_hashes {
                // What happen if the blobs also exists linked to another
                // transaction?
                self.inner.delete_cf(self.ledger_blobs_cf, hash)?;
            }
            self.inner.delete_cf(
                self.ledger_blobs_height_cf,
                block_height.to_be_bytes(),
            )?;
        }

        Ok(())
    }

    fn blobs_by_height(
        &self,
        block_height: u64,
    ) -> Result<Option<Vec<[u8; 32]>>> {
        let blob_hashes_bytes = self
            .inner
            .get_cf(self.ledger_blobs_height_cf, block_height.to_be_bytes())?;

        if let Some(blob_hashes_bytes) = blob_hashes_bytes {
            let mut blob_hashes = vec![];
            for chunk in blob_hashes_bytes.chunks(32) {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(chunk);
                blob_hashes.push(hash);
            }
            Ok(Some(blob_hashes))
        } else {
            Ok(None)
        }
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
                    let mut tx = SpentTransaction::read(&mut &buf[..])?;
                    if let Some(blobs) = tx.inner.inner.blob_mut() {
                        for blob in blobs {
                            // Retrieve blob data from the ledger
                            let sidecar = self
                                .blob_data_by_hash(&blob.hash)?
                                .map(|bytes| {
                                    BlobSidecar::from_buf(&mut &bytes[..])
                                })
                                .transpose()
                                .map_err(
                                    error::RocksDbError::blob_sidecar_parse,
                                )?;
                            blob.data = sidecar;
                        }
                    }
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
            let opt_blob = result.map_err(std::io::Error::other)?;

            let Some(blob) = opt_blob else {
                return Err(
                    error::RocksDbError::MissingLedgerTransaction.into()
                );
            };

            let stx = SpentTransaction::read(&mut &blob[..])?;

            spent_transactions.push(stx);
        }

        Ok(spent_transactions)
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
