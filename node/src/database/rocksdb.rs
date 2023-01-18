// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Registry, Tx, DB};
use anyhow::Result;

enum TxType {
    ReadWrite,
    ReadOnly,
}

pub struct Backend {}

impl Backend {
    fn begin_tx(&mut self, read_only: TxType) -> Transaction {
        // TODO: rusk/issues/806 This should be addressed with another issue
        // about integrating RocksDB
        Transaction {}
    }
}

impl DB for Backend {
    type T = Transaction;

    fn view<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&Transaction) -> Result<()>,
    {
        // Create read-only transaction
        let mut tx = self.begin_tx(TxType::ReadOnly);

        // If f returns err, no commit will be applied into backend
        // storage
        f(&tx)?;

        // Release tx resources
        tx.close();

        Ok(())
    }

    fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&Transaction) -> Result<()>,
    {
        // Create read-write transaction
        let mut tx = self.begin_tx(TxType::ReadWrite);

        // If f returns err, no commit will be applied into backend
        // storage
        f(&tx)?;

        // Apply changes
        tx.commit()?;

        // Release tx resources
        tx.close();

        Ok(())
    }

    fn close(&mut self) {}
}

pub struct Transaction {}

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

    fn clear_candidate_messages(&mut self) -> Result<()> {
        anyhow::Ok(())
    }

    fn clear_database(&mut self) -> Result<()> {
        anyhow::Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        anyhow::Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        anyhow::Ok(())
    }

    fn close(&mut self) {}

    // Read-write transactions
    // fn store_block(&mut self, block: &Block, persisted)
}
