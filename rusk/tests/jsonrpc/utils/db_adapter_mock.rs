// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::{self as node_ledger};
use node_data::message::{payload as node_payload, ConsensusHeader};

use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
use rusk::jsonrpc::infrastructure::error::DbError;
use rusk::jsonrpc::model;
use tempfile::tempdir;

use std::collections::HashMap;
use std::fmt::Debug;

/// A mock implementation of `DatabaseAdapter` for testing purposes.
/// Stores data in HashMaps.
#[derive(Debug, Clone, Default)]
pub struct MockDbAdapter {
    /// Mock storage for blocks keyed by height.
    pub blocks_by_height: HashMap<u64, model::block::Block>,
    /// Mock storage for blocks keyed by hex-encoded hash.
    pub blocks_by_hash: HashMap<String, model::block::Block>,
    /// Mock storage for headers keyed by height.
    pub headers_by_height: HashMap<u64, model::block::BlockHeader>,
    /// Mock storage for headers keyed by hex-encoded hash.
    pub headers_by_hash: HashMap<String, model::block::BlockHeader>,
    /// Mock storage for spent transactions keyed by hex-encoded hash.
    pub spent_txs_by_hash: HashMap<String, node_ledger::SpentTransaction>,
    /// Mock storage for mempool transactions keyed by tx_id.
    pub mempool_txs: HashMap<[u8; 32], node_ledger::Transaction>,
    /// Mock storage for candidate blocks keyed by block hash.
    pub candidates_by_hash: HashMap<[u8; 32], node_ledger::Block>,
    /// Mock storage for candidate blocks keyed by consensus header
    /// (serialized?). Using String representation for simplicity in mock.
    pub candidates_by_iteration: HashMap<String, node_ledger::Block>,
    /// Mock storage for candidate blocks count
    pub candidate_blocks_count: u64,
    /// Mock storage for validation results keyed by consensus header
    /// (serialized?). Using String representation for simplicity in mock.
    pub validation_results: HashMap<String, node_payload::ValidationResult>,
    /// Mock storage for metadata keyed by byte vector.
    pub metadata: HashMap<Vec<u8>, Vec<u8>>,
    /// The height considered "latest" by this mock (used by some old tests,
    /// may need removal).
    pub latest_height: u64,
    /// Optional error to return from all methods.
    pub force_error: Option<DbError>,
}

#[async_trait::async_trait]
impl DatabaseAdapter for MockDbAdapter {
    // --- Required Primitive Methods --- //

    // --- Ledger Primitives ---

    /// Returns a predefined block based on hash, or `force_error` if set.
    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
        _include_txs: bool,
    ) -> Result<Option<model::block::Block>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Simple mock logic: check hash validity roughly
        if block_hash_hex.len() != 64 || hex::decode(block_hash_hex).is_err() {
            return Err(DbError::QueryFailed(
                "Invalid block hash format/length".into(),
            ));
        }
        Ok(self.blocks_by_hash.get(block_hash_hex).cloned())
    }

    async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Simple mock: Return transactions associated with the block if found.
        // Assumes `Block` struct in the mock has transactions pre-populated.
        match self.blocks_by_hash.get(block_hash_hex) {
            Some(block) => Ok(block
                .transactions // Assumes Block::transactions is
                // Option<Vec<TransactionResponse>>
                .clone()),
            None => Ok(None),
        }
    }

    async fn get_block_faults_by_hash(
        &self,
        _block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockFaults>, DbError> {
        Box::pin(async move {
            if let Some(err) = self.force_error.clone() {
                return Err(err);
            }
            Ok(None)
        })
        .await
    }

    async fn get_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<String>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Find block by height and return its hash
        Ok(self
            .blocks_by_height
            .get(&height)
            .map(|b| b.header.hash.clone()))
    }

    /// Returns a predefined block header based on hash, or `force_error` if
    /// set.
    async fn get_block_header_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockHeader>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Simple mock logic: check hash validity roughly
        if block_hash_hex.len() != 64 || hex::decode(block_hash_hex).is_err() {
            return Err(DbError::QueryFailed(
                "Invalid block hash format/length".into(),
            ));
        }
        Ok(self.headers_by_hash.get(block_hash_hex).cloned())
    }

    async fn get_block_status_by_height(
        &self,
        _height: u64,
    ) -> Result<Option<model::block::BlockStatus>, DbError> {
        Box::pin(async move {
            if let Some(err) = self.force_error.clone() {
                return Err(err);
            }
            Ok(None)
        })
        .await
    }

    async fn get_spent_transaction_by_hash(
        &self,
        _tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionInfo>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Return None. Constructing TransactionInfo is too complex for
        // a simple mock.
        Ok(None)
    }

    async fn ledger_tx_exists(
        &self,
        tx_id: &[u8; 32],
    ) -> Result<bool, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Check if a SpentTransaction exists for this hash
        let exists = self
            .spent_txs_by_hash
            .values()
            .any(|stx| stx.inner.id() == *tx_id);
        Ok(exists)
    }

    // --- Mempool Primitives ---

    async fn mempool_tx(
        &self,
        tx_id: [u8; 32],
    ) -> Result<Option<model::transaction::TransactionResponse>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Retrieve NodeTransaction, convert if found.
        Ok(self
            .mempool_txs
            .get(&tx_id)
            .cloned()
            .map(model::transaction::TransactionResponse::from))
    }

    async fn mempool_tx_exists(
        &self,
        tx_id: [u8; 32],
    ) -> Result<bool, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.mempool_txs.contains_key(&tx_id))
    }

    async fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Retrieve all, sort by fee, convert.
        let mut txs: Vec<_> = self.mempool_txs.values().cloned().collect();
        txs.sort_by_key(|b| std::cmp::Reverse(b.gas_price()));
        Ok(txs
            .into_iter()
            .map(model::transaction::TransactionResponse::from)
            .collect())
    }

    async fn mempool_txs_count(&self) -> Result<usize, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.mempool_txs.len())
    }

    async fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Sort stored mempool txs by gas_price (descending)
        let mut tx_fees: Vec<_> = self
            .mempool_txs
            .values()
            .map(|tx| (tx.gas_price(), tx.id()))
            .collect();
        tx_fees.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by fee descending
        Ok(tx_fees)
    }

    async fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Sort stored mempool txs by gas_price (ascending)
        let mut tx_fees: Vec<_> = self
            .mempool_txs
            .values()
            .map(|tx| (tx.gas_price(), tx.id()))
            .collect();
        tx_fees.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by fee ascending
        Ok(tx_fees)
    }

    // --- ConsensusStorage Primitives ---

    async fn candidate(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Retrieve node_ledger::Block, convert if found.
        Ok(self
            .candidates_by_hash
            .get(hash)
            .cloned()
            .map(model::block::CandidateBlock::from))
    }

    async fn candidate_by_iteration(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Serialize header to use as key (simplified mock)
        let key = format!(
            "{:?}_{}_{}",
            header.prev_block_hash, header.round, header.iteration
        );
        Ok(self
            .candidates_by_iteration
            .get(&key)
            .cloned()
            .map(model::block::CandidateBlock::from))
    }

    async fn validation_result(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::consensus::ValidationResult>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Serialize header to use as key (simplified mock)
        let key = format!(
            "{:?}_{}_{}",
            header.prev_block_hash, header.round, header.iteration
        );
        Ok(self
            .validation_results
            .get(&key)
            .cloned()
            .map(model::consensus::ValidationResult::from))
    }

    // --- Metadata Primitives ---

    async fn metadata_op_read(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.metadata.get(key).cloned())
    }

    async fn metadata_op_write(
        &self,
        _key: &[u8],
        _value: &[u8],
    ) -> Result<(), DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Silently ignore write as we changed to &self for trait
        // compatibility If mutation is needed, the HashMap would need
        // interior mutability (e.g., Arc<RwLock<...>>) For simple
        // tests, ignoring the write is often acceptable.
        Ok(())
    }

    async fn get_block_finality_status(
        &self,
        block_hash_hex: &str,
    ) -> Result<model::block::BlockFinalityStatus, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Simple mock: return Accepted if block exists in map, otherwise
        // Unknown
        if self.blocks_by_hash.contains_key(block_hash_hex)
            || self.headers_by_hash.contains_key(block_hash_hex)
        {
            Ok(model::block::BlockFinalityStatus::Accepted)
        } else {
            Ok(model::block::BlockFinalityStatus::Unknown)
        }
    }

    async fn get_block_by_height(
        &self,
        height: u64,
        _include_txs: bool,
    ) -> Result<Option<model::block::Block>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.blocks_by_height.get(&height).cloned())
    }

    async fn get_latest_block(
        &self,
        _include_txs: bool,
    ) -> Result<model::block::Block, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        self.blocks_by_height
            .get(&self.latest_height)
            .cloned()
            .ok_or_else(|| {
                DbError::NotFound("Tip block header not found".into())
            })
    }

    async fn get_candidate_blocks_count(&self) -> Result<u64, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.candidate_blocks_count)
    }
}

/// Helper to setup a temporary RocksDB database
#[cfg(feature = "chain")]
pub(crate) fn setup_test_db(
) -> (tempfile::TempDir, ::node::database::rocksdb::Backend) {
    let temp_dir = tempdir().expect("Failed to create temp dir for DB");
    let db_opts = ::node::database::DatabaseOptions {
        create_if_missing: true, // Ensure DB is created
        ..Default::default()
    };
    // Call create_or_open via the DB trait
    let db = ::node::database::DB::create_or_open(temp_dir.path(), db_opts);
    (temp_dir, db)
}
