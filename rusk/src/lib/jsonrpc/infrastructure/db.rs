// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Database Adapter Infrastructure
//!
//! This module defines the [`DatabaseAdapter`] trait,
//! along with its concrete implementation ([`RuskDbAdapter`]) for interacting
//! with the Rusk node's underlying data sources.
//!
//! See the [`DatabaseAdapter` trait documentation](#trait.DatabaseAdapter) for
//! design rationale and usage.

use crate::jsonrpc::infrastructure::error::DbError;
use crate::jsonrpc::model::{self, gas::*};

use async_trait::async_trait;
use futures::future::{join_all, try_join_all};
use futures::try_join;
use hex;

use node::database::rocksdb::{MD_HASH_KEY, MD_LAST_ITER};
use node::database::{ConsensusStorage, Ledger, Mempool, Metadata, DB};

use node_data::ledger;
use node_data::message::ConsensusHeader;
use node_data::Serializable;

use std::fmt::Debug;
use std::sync::Arc;

/// Defines the interface for accessing the Rusk node's data layer.
///
/// This trait abstracts the underlying database implementation (e.g., RocksDB
/// via `node::database::rocksdb::Backend`) and provides a structured way for
/// JSON-RPC services to query blockchain state, mempool, and consensus data.
///
/// # Design
///
/// The trait requires implementations for a minimal set of **required primitive
/// methods** that directly map to underlying database operations (like fetching
/// a raw block, reading metadata, or querying the mempool).
///
/// Building upon these required primitives, the trait provides **default
/// implementations** for more complex queries (like getting a block by height,
/// retrieving full transaction info, or calculating gas stats). These default
/// methods combine calls to the required primitives, encapsulating data joining
/// and transformation logic.
///
/// This design offers:
/// - **Testability:** Mocks only need to implement the primitive required
///   methods.
/// - **Flexibility:** Underlying storage can change without breaking RPC
///   handlers if the primitives are maintained.
/// - **Clear Separation:** Primitive data access is separated from data
///   composition logic.
///
/// # Errors
///
/// Methods return `Result<_, DbError>` to handle potential issues like database
/// errors, data not found, or invalid input.
///
/// # Implementations
///
/// - [`RuskDbAdapter`]: The concrete implementation using the node's RocksDB
///   backend.
/// - `MockDbAdapter`: (Typically found in `rusk/tests/`) Used for testing.
#[async_trait]
pub trait DatabaseAdapter: Send + Sync + Debug + 'static {
    // --- Required Primitive Methods --- //

    // --- Ledger Primitives ---

    /// (Required) Retrieves a block summary by its 32-byte hash.
    ///
    /// Corresponds to `Ledger::block` and potentially combining with
    /// `Ledger::block_label_by_height`.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::Block>)` if the block is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, DbError>;

    /// (Required) Retrieves the list of full transactions for a block by hash.
    ///
    /// Corresponds to iterating `Ledger::block(...).txs()`.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)` if the
    ///   transactions are found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>;

    /// (Required) Retrieves consensus faults for a block by hash.
    ///
    /// Corresponds to iterating `Ledger::block(...).faults()`.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockFaults>)` if the faults are found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_faults_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockFaults>, DbError>;

    /// (Required) Retrieves a block hash hex string by its height.
    ///
    /// Corresponds to `Ledger::block_hash_by_height`.
    ///
    /// # Arguments
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<String>)` if the hash is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<String>, DbError>;

    /// (Required) Retrieves a block header by its 32-byte hash.
    ///
    /// Corresponds to `Ledger::block_header`.
    ///
    /// # Arguments
    /// * `block_hash_hex`: 64-char hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockHeader>)` if the header is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_header_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockHeader>, DbError>;

    /// (Required) Retrieves the consensus label for a block by height.
    ///
    /// Corresponds to `Ledger::block_label_by_height`.
    ///
    /// # Arguments
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockLabel>)` if the label is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockLabel>, DbError>;

    /// (Required) Retrieves a spent transaction record by its hash.
    ///
    /// Corresponds to `Ledger::ledger_tx`.
    ///
    /// # Arguments
    /// * `tx_hash_hex`: 64-char hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionInfo>)` if the transaction
    ///   is found and its context retrieved.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_spent_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionInfo>, DbError>;

    /// (Required) Checks if a transaction exists in the confirmed ledger.
    ///
    /// Corresponds to `Ledger::ledger_tx_exists`.
    ///
    /// # Arguments
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` if the transaction exists.
    /// * `Err(DbError)` if a database error occurs.
    async fn ledger_tx_exists(&self, tx_id: &[u8; 32])
        -> Result<bool, DbError>;

    // --- Mempool Primitives ---

    /// (Required) Retrieves a transaction from the mempool by its hash.
    ///
    /// Corresponds to `Mempool::mempool_tx`.
    ///
    /// # Arguments
    ///
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionResponse>)` if the
    ///   transaction is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_tx(
        &self,
        tx_id: [u8; 32],
    ) -> Result<Option<model::transaction::TransactionResponse>, DbError>;

    /// (Required) Checks if a transaction exists in the mempool.
    ///
    /// Corresponds to `Mempool::mempool_tx_exists`.
    ///
    /// # Arguments
    ///
    /// * `tx_id`: 32-byte transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` if the transaction exists.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_tx_exists(&self, tx_id: [u8; 32])
        -> Result<bool, DbError>;

    /// (Required) Gets an iterator over mempool transactions, sorted by fee
    /// (highest first).
    ///
    /// Corresponds to `Mempool::mempool_txs_sorted_by_fee`.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::transaction::TransactionResponse>)` representing
    ///   mempool transactions.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, DbError>;

    /// (Required) Gets the current count of transactions in the mempool.
    ///
    /// Corresponds to `Mempool::mempool_txs_count`.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` if the count is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_txs_count(&self) -> Result<usize, DbError>;

    /// (Required) Gets an iterator over mempool (fee, tx_id) pairs, sorted by
    /// fee (highest first).
    ///
    /// Corresponds to `Mempool::mempool_txs_ids_sorted_by_fee`.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(u64, [u8; 32])>)` if the iterator is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError>;

    /// (Required) Gets an iterator over mempool (fee, tx_id) pairs, sorted by
    /// fee (lowest first).
    ///
    /// Corresponds to `Mempool::mempool_txs_ids_sorted_by_low_fee`.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(u64, [u8; 32])>)` if the iterator is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError>;

    // --- ConsensusStorage Primitives ---

    /// (Required) Retrieves a candidate block by its hash.
    ///
    /// Corresponds to `ConsensusStorage::candidate`.
    ///
    /// # Arguments
    /// * `hash`: 32-byte candidate block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    async fn candidate(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<model::block::CandidateBlock>, DbError>;

    /// (Required) Retrieves a candidate block by its consensus header.
    ///
    /// Corresponds to `ConsensusStorage::candidate_by_iteration`.
    ///
    /// # Arguments
    /// * `header`: Consensus header.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    async fn candidate_by_iteration(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::block::CandidateBlock>, DbError>;

    /// (Required) Retrieves a validation result by its consensus header.
    ///
    /// Corresponds to `ConsensusStorage::validation_result`.
    ///
    /// # Arguments
    ///
    /// * `header`: Consensus header.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::consensus::ValidationResult>)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    async fn validation_result(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::consensus::ValidationResult>, DbError>;

    // --- Metadata Primitives ---

    /// (Required) Reads a value from the metadata storage by key.
    ///
    /// Corresponds to `Metadata::op_read`.
    ///
    /// # Arguments
    ///
    /// * `key`: Key to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<u8>>)` if the key is found.
    /// * `Err(DbError)` if a database error occurs.
    async fn metadata_op_read(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, DbError>;

    /// (Required) Writes a value to the metadata storage by key.
    ///
    /// Corresponds to `Metadata::op_write`.
    ///
    /// # Arguments
    ///
    /// * `key`: Key to write.
    /// * `value`: Value to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the value is written.
    /// * `Err(DbError)` if a database error occurs.
    async fn metadata_op_write(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), DbError>;

    // --- Default Implementations --- //

    /// (Default) Retrieves the height of the current chain tip.
    ///
    /// Implementation uses `metadata_op_read` for tip hash and
    /// `get_block_header_by_hash`.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` if the height is found.
    /// * `Err(DbError)` if the tip hash is not found or the block header is not
    ///   found.
    async fn get_block_height(&self) -> Result<u64, DbError> {
        let tip_hash_bytes = self.metadata_op_read(MD_HASH_KEY).await?.ok_or(
            DbError::NotFound("Tip hash metadata key not found".into()),
        )?;
        let tip_hash: [u8; 32] = tip_hash_bytes.try_into().map_err(|_| {
            DbError::InternalError("Invalid tip hash length in metadata".into())
        })?;
        let header = self
            .get_block_header_by_hash(&hex::encode(tip_hash))
            .await?
            .ok_or(DbError::NotFound("Tip block header not found".into()))?;
        Ok(header.height)
    }

    /// (Default) Retrieves a candidate block by its hash, converting to model.
    ///
    /// Implementation uses required `candidate`.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: Hex string of the block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::CandidateBlock>)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    async fn get_candidate_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;
        self.candidate(&block_hash).await
    }

    /// (Default) Retrieves the latest candidate block proposed during
    /// consensus.
    ///
    /// Implementation uses `metadata_op_read` and required
    /// `candidate_by_iteration`.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::CandidateBlock)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    async fn get_latest_candidate_block(
        &self,
    ) -> Result<model::block::CandidateBlock, DbError> {
        let latest_header_bytes =
            self.metadata_op_read(MD_LAST_ITER).await?.ok_or(
                DbError::NotFound("Last iteration metadata not found".into()),
            )?;
        let latest_header =
            ConsensusHeader::read(&mut latest_header_bytes.as_slice())
                .map_err(|e| {
                    DbError::InternalError(format!(
                        "Failed to deserialize header: {}",
                        e
                    ))
                })?;
        self.candidate_by_iteration(&latest_header)
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Candidate block not found for header: {:?}",
                    latest_header
                ))
            })
    }

    /// (Default) Retrieves a validation result by consensus header, converting
    /// to model.
    ///
    /// # Arguments
    ///
    /// * `prev_block_hash_hex`: Hex string of the previous block hash for the
    ///   consensus round.
    /// * `round`: The consensus round number (block height).
    /// * `iteration`: The consensus iteration number.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(model::consensus::ValidationResult))` if found.
    /// * `Ok(None)` if no validation result matches the identifier.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    /// Implementation uses required `validation_result`.
    async fn get_validation_result(
        &self,
        prev_block_hash_hex: &str,
        round: u64,
        iteration: u8,
    ) -> Result<Option<model::consensus::ValidationResult>, DbError> {
        let prev_block_hash: [u8; 32] = hex::decode(prev_block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!(
                    "Invalid prev block hash hex: {}",
                    e
                ))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid prev block hash length".into())
            })?;
        let header = ConsensusHeader {
            prev_block_hash,
            round,
            iteration,
        };
        self.validation_result(&header).await
    }

    /// (Default) Retrieves the latest validation result.
    ///
    /// # Returns
    ///
    /// * `Ok(model::consensus::ValidationResult)` if found.
    /// * `Err(DbError)` if the identifier is invalid or a database error
    ///   occurs.
    ///
    /// Implementation uses `metadata_op_read` and required `validation_result`.
    async fn get_latest_validation_result(
        &self,
    ) -> Result<model::consensus::ValidationResult, DbError> {
        let latest_header_bytes =
            self.metadata_op_read(MD_LAST_ITER).await?.ok_or(
                DbError::NotFound("Last iteration metadata not found".into()),
            )?;
        let latest_header =
            ConsensusHeader::read(&mut latest_header_bytes.as_slice())
                .map_err(|e| {
                    DbError::InternalError(format!(
                        "Failed to deserialize header: {}",
                        e
                    ))
                })?;
        self.validation_result(&latest_header)
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Validation result not found for header: {:?}",
                    latest_header
                ))
            })
    }

    /// (Default) Retrieves the status (Confirmed, Pending, NotFound) of a
    /// transaction.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(model::transaction::TransactionStatus)` describing the status.
    /// * `Err(DbError)` if the hash format is invalid, the transaction is not
    ///   found (neither confirmed nor pending), or a database error occurs.
    ///
    /// Implementation uses `ledger_tx_exists`, `mempool_tx_exists`,
    /// and the refactored `get_spent_transaction_by_hash`.
    async fn get_transaction_status(
        &self,
        tx_hash_hex: &str,
    ) -> Result<model::transaction::TransactionStatus, DbError> {
        let tx_id: [u8; 32] = hex::decode(tx_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid tx hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid tx hash length".into())
            })?;

        if self.ledger_tx_exists(&tx_id).await? {
            match self.get_spent_transaction_by_hash(tx_hash_hex).await? {
                Some(tx_info) => {
                    let status_type = if tx_info.error.is_some() {
                        model::transaction::TransactionStatusType::Failed
                    } else {
                        model::transaction::TransactionStatusType::Executed
                    };

                    Ok(model::transaction::TransactionStatus {
                        status: status_type,
                        block_height: Some(tx_info.block_height),
                        block_hash: Some(tx_info.block_hash),
                        gas_spent: Some(tx_info.gas_spent),
                        timestamp: Some(tx_info.timestamp),
                        error: tx_info.error,
                    })
                }
                None => Err(DbError::InternalError(format!(
                    "Tx {} exists in ledger but TransactionInfo not found",
                    tx_hash_hex
                ))),
            }
        } else if self.mempool_tx_exists(tx_id).await? {
            Ok(model::transaction::TransactionStatus {
                status: model::transaction::TransactionStatusType::Pending,
                block_height: None,
                block_hash: None,
                gas_spent: None,
                timestamp: None,
                error: None,
            })
        } else {
            Err(DbError::NotFound(format!(
                "Transaction {} not found",
                tx_hash_hex
            )))
        }
    }

    /// (Default) Retrieves a list of transactions currently in the mempool.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::transaction::TransactionResponse>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    ///
    /// Implementation uses required `mempool_txs_sorted_by_fee`.
    async fn get_mempool_transactions(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, DbError> {
        self.mempool_txs_sorted_by_fee().await
    }

    /// (Default) Retrieves a specific transaction from the mempool by hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Hex string of the transaction hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionResponse>)` if found.
    /// * `Err(DbError)` if the hash format is invalid or a database error
    ///   occurs.
    ///
    /// Implementation uses required `mempool_tx`.
    async fn get_mempool_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionResponse>, DbError> {
        let tx_id: [u8; 32] = hex::decode(tx_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid tx hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid tx hash length".into())
            })?;
        self.mempool_tx(tx_id).await
    }

    /// (Default) Retrieves statistics about the mempool (count, fee range).
    ///
    /// # Returns
    ///
    /// * `Ok(model::mempool::MempoolInfo)` if found.
    /// * `Err(DbError)` if a database error occurs.
    ///
    /// Implementation uses required `mempool_txs_count`,
    /// `mempool_txs_ids_sorted_by_fee`, `mempool_txs_ids_sorted_by_low_fee`.
    ///
    /// This implementation uses `try_join` to execute the three required
    /// method calls concurrently.
    async fn get_mempool_info(
        &self,
    ) -> Result<model::mempool::MempoolInfo, DbError> {
        let (count_res, fee_high_res, fee_low_res) = try_join!(
            self.mempool_txs_count(),
            self.mempool_txs_ids_sorted_by_fee(),
            self.mempool_txs_ids_sorted_by_low_fee()
        )?;

        let count = count_res as u64;
        let max_fee = fee_high_res.first().map(|(fee, _)| *fee);
        let min_fee = fee_low_res.first().map(|(fee, _)| *fee);

        Ok(model::mempool::MempoolInfo {
            count,
            max_fee,
            min_fee,
        })
    }

    /// (Default) Retrieves overall chain statistics.
    ///
    /// # Returns
    ///
    /// * `Ok(model::chain::ChainStats)` if found.
    /// * `Err(DbError)` if a database error occurs.
    ///
    /// Implementation uses `get_latest_block_header`.
    async fn get_chain_stats(
        &self,
    ) -> Result<model::chain::ChainStats, DbError> {
        let latest_header = self.get_latest_block_header().await?;
        Ok(model::chain::ChainStats {
            height: latest_header.height,
            tip_hash: latest_header.hash,
            state_root_hash: latest_header.state_hash,
        })
    }

    /// (Default) Calculates gas price statistics based on mempool fees.
    ///
    /// # Arguments
    ///
    /// * `max_transactions`: Maximum number of transactions to consider.
    ///
    /// # Returns
    ///
    /// * `Ok(model::gas::GasPriceStats)` if found.
    /// * `Err(DbError)` if a database error occurs.
    ///
    /// Implementation uses `mempool_txs_ids_sorted_by_fee`.
    async fn get_gas_price(
        &self,
        max_transactions: Option<usize>,
    ) -> Result<model::gas::GasPriceStats, DbError> {
        let mut prices = self.mempool_txs_ids_sorted_by_fee().await?;

        if let Some(max) = max_transactions {
            prices.truncate(max);
        }

        let gas_prices: Vec<u64> = prices.into_iter().map(|(p, _)| p).collect();

        if gas_prices.is_empty() {
            Ok(GasPriceStats {
                average: 1,
                max: 1,
                median: 1,
                min: 1,
            })
        } else {
            let count = gas_prices.len() as u64;
            let sum: u64 = gas_prices.iter().sum();
            let average = (sum + count - 1) / count;
            let max = *gas_prices.first().unwrap_or(&1);
            let min = *gas_prices.last().unwrap_or(&1);

            let mut sorted_prices = gas_prices;
            sorted_prices.sort_unstable();
            let mid = sorted_prices.len() / 2;
            let median = if sorted_prices.len() % 2 == 0 {
                (sorted_prices[mid - 1] + sorted_prices[mid]) / 2
            } else {
                sorted_prices[mid]
            };

            Ok(GasPriceStats {
                average,
                max,
                median,
                min,
            })
        }
    }

    /// (Default) Suggests gas price statistics based on mempool fees.
    ///
    /// This method simply delegates to [`get_gas_price`](Self::get_gas_price).
    ///
    /// # Arguments
    ///
    /// * `max_transactions`: Maximum number of transactions to consider.
    ///
    /// # Returns
    ///
    /// * `Ok(model::gas::GasPriceStats)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn suggest_transaction_fee(
        &self,
        max_transactions: Option<usize>,
    ) -> Result<model::gas::GasPriceStats, DbError> {
        self.get_gas_price(max_transactions).await
    }

    // --- Passthrough Default Implementations ---
    // These call other default or required methods without much extra logic.

    /// (Default) Retrieves block summary by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::Block>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::Block>, DbError> {
        match self.get_block_hash_by_height(height).await? {
            Some(hash) => self.get_block_by_hash(&hash).await,
            None => Ok(None),
        }
    }

    /// (Default) Retrieves the latest block summary.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::Block)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_latest_block(&self) -> Result<model::block::Block, DbError> {
        let height = self.get_block_height().await?;
        self.get_block_by_height(height).await?.ok_or_else(|| {
            DbError::NotFound(format!(
                "Latest block not found at height {}",
                height
            ))
        })
    }

    /// (Default) Retrieves a range of block summaries concurrently.
    ///
    /// # Arguments
    ///
    /// * `height_start`: Start height of the range.
    /// * `height_end`: End height of the range.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::Block>)` containing summaries for found blocks
    ///   in the range. Note: If individual block lookups within the range fail
    ///   (e.g., height not found), they are skipped.
    /// * `Err(DbError::InternalError)` if `height_start > height_end`.
    async fn get_blocks_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::Block>, DbError> {
        if height_start > height_end {
            return Err(DbError::InternalError(
                "Start height cannot be greater than end height".into(),
            ));
        }
        let futures =
            (height_start..=height_end).map(|h| self.get_block_by_height(h));
        let results: Vec<Result<Option<model::block::Block>, DbError>> =
            join_all(futures).await;
        results.into_iter().filter_map(Result::transpose).collect()
    }

    /// (Default) Retrieves multiple block summaries concurrently.
    ///
    /// # Arguments
    ///
    /// * `hashes_hex`: Array of block hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::block::Block>>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_blocks_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::Block>>, DbError> {
        let futures = hashes_hex.iter().map(|h| self.get_block_by_hash(h));
        try_join_all(futures).await
    }

    /// (Default) Retrieves the latest block header.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::BlockHeader)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_latest_block_header(
        &self,
    ) -> Result<model::block::BlockHeader, DbError> {
        let block = self.get_latest_block().await?;
        Ok(block.header)
    }

    /// (Default) Retrieves block header by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockHeader>)` if the header is found for the
    ///   given height.
    /// * `Err(DbError)` if a database error occurs during hash or header
    ///   lookup.
    async fn get_block_header_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockHeader>, DbError> {
        match self.get_block_hash_by_height(height).await? {
            Some(hash) => self.get_block_header_by_hash(&hash).await,
            None => Ok(None),
        }
    }

    /// (Default) Retrieves a range of block headers concurrently.
    ///
    /// # Arguments
    ///
    /// * `height_start`: Start height of the range.
    /// * `height_end`: End height of the range.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::BlockHeader>)` containing headers for found
    ///   blocks in the range. Note: If individual header lookups within the
    ///   range fail (e.g., height not found), they are skipped.
    /// * `Err(DbError::InternalError)` if `height_start > height_end`.
    async fn get_block_headers_range(
        &self,
        height_start: u64,
        height_end: u64,
    ) -> Result<Vec<model::block::BlockHeader>, DbError> {
        if height_start > height_end {
            return Err(DbError::InternalError(
                "Start height cannot be greater than end height".into(),
            ));
        }
        let futures = (height_start..=height_end)
            .map(|h| self.get_block_header_by_height(h));
        let results: Vec<Result<Option<model::block::BlockHeader>, DbError>> =
            join_all(futures).await;
        results.into_iter().filter_map(Result::transpose).collect()
    }

    /// (Default) Retrieves multiple block headers concurrently.
    ///
    /// # Arguments
    ///
    /// * `hashes_hex`: Array of block hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::block::BlockHeader>>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_headers_by_hashes(
        &self,
        hashes_hex: &[String],
    ) -> Result<Vec<Option<model::block::BlockHeader>>, DbError> {
        let futures =
            hashes_hex.iter().map(|h| self.get_block_header_by_hash(h));
        try_join_all(futures).await
    }

    /// (Default) Retrieves block timestamp by hash.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: Block hash.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<u64>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_timestamp_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<u64>, DbError> {
        Ok(self
            .get_block_header_by_hash(block_hash_hex)
            .await?
            .map(|h| h.timestamp))
    }

    /// (Default) Retrieves block timestamp by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<u64>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_timestamp_by_height(
        &self,
        height: u64,
    ) -> Result<Option<u64>, DbError> {
        Ok(self
            .get_block_header_by_height(height)
            .await?
            .map(|h| h.timestamp))
    }

    /// (Default) Retrieves transactions for a block by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<model::transaction::TransactionResponse>>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_transactions_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        match self.get_block_hash_by_height(height).await? {
            Some(hash) => self.get_block_transactions_by_hash(&hash).await,
            None => Ok(None),
        }
    }

    /// (Default) Retrieves faults for a block by height.
    ///
    /// # Arguments
    ///
    /// * `height`: Height of the block.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::block::BlockFaults>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_block_faults_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockFaults>, DbError> {
        match self.get_block_hash_by_height(height).await? {
            Some(hash) => self.get_block_faults_by_hash(&hash).await,
            None => Ok(None),
        }
    }

    /// (Default) Retrieves the consensus label for the latest block.
    ///
    /// # Returns
    ///
    /// * `Ok(model::block::BlockLabel)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_latest_block_label(
        &self,
    ) -> Result<model::block::BlockLabel, DbError> {
        let height = self.get_block_height().await?;
        self.get_block_label_by_height(height)
            .await?
            .ok_or_else(|| {
                DbError::NotFound(format!(
                    "Label not found for latest block {}",
                    height
                ))
            })
    }

    /// (Default) Retrieves detailed transaction info by hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash_hex`: Transaction hash.
    /// * `include_tx_index`: Whether to include the transaction index in the
    ///   returned [`TransactionInfo`].
    ///
    /// # Returns
    ///
    /// * `Ok(Option<model::transaction::TransactionInfo>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    ///
    /// This method now primarily relies on the refactored primitive
    /// `get_spent_transaction_by_hash` which returns `TransactionInfo`.
    /// If `include_tx_index` is true, it fetches block transactions to find
    /// the index.
    async fn get_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
        include_tx_index: bool,
    ) -> Result<Option<model::transaction::TransactionInfo>, DbError> {
        let tx_info_opt =
            self.get_spent_transaction_by_hash(tx_hash_hex).await?;

        if let Some(mut tx_info) = tx_info_opt {
            if include_tx_index && tx_info.tx_index.is_none() {
                if let Some(txs) = self
                    .get_block_transactions_by_hash(&tx_info.block_hash)
                    .await?
                {
                    tx_info.tx_index = txs
                        .iter()
                        .position(|tx| tx.base.tx_hash == tx_hash_hex)
                        .map(|i| i as u32);
                }
            }
            Ok(Some(tx_info))
        } else {
            Ok(None)
        }
    }

    /// (Default) Retrieves multiple transactions concurrently.
    ///
    /// # Arguments
    ///
    /// * `tx_hashes_hex`: Array of transaction hashes.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Option<model::transaction::TransactionInfo>>)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_transactions_batch(
        &self,
        tx_hashes_hex: &[String],
    ) -> Result<Vec<Option<model::transaction::TransactionInfo>>, DbError> {
        let futures = tx_hashes_hex
            .iter()
            .map(|h_str| self.get_transaction_by_hash(h_str, false));
        try_join_all(futures).await
    }

    /// (Default) Retrieves the count of transactions currently in the mempool.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` if found.
    /// * `Err(DbError)` if a database error occurs.
    async fn get_mempool_transactions_count(&self) -> Result<u64, DbError> {
        let info = self.get_mempool_info().await?;
        Ok(info.count)
    }

    /// (Default) Retrieves the `count` most recent block summaries.
    ///
    /// # Arguments
    ///
    /// * `count`: The number of latest blocks to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<model::block::Block>)` containing the block summaries.
    /// * `Err(DbError)` if fetching the latest block height or the block range
    ///   fails.
    async fn get_latest_blocks(
        &self,
        count: u64,
    ) -> Result<Vec<model::block::Block>, DbError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let latest_height = self.get_block_height().await?;
        let start_height = latest_height.saturating_sub(count - 1);
        self.get_blocks_range(start_height, latest_height).await
    }

    /// (Default) Retrieves the total number of blocks in the chain.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` containing the block count (latest height + 1).
    /// * `Err(DbError)` if fetching the latest block height fails.
    async fn get_blocks_count(&self) -> Result<u64, DbError> {
        let latest_height = self.get_block_height().await?;
        Ok(latest_height + 1)
    }

    /// (Default) Retrieves a pair of consecutive block summaries by the height
    /// of the first block.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the first block in the pair.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((Block, Block)))` if both blocks at `height` and `height + 1`
    ///   are found.
    /// * `Ok(None)` if either block in the pair is not found.
    /// * `Err(DbError)` if a database error occurs during lookup.
    async fn get_block_pair(
        &self,
        height: u64,
    ) -> Result<Option<(model::block::Block, model::block::Block)>, DbError>
    {
        let block1_fut = self.get_block_by_height(height);
        let block2_fut = self.get_block_by_height(height.saturating_add(1));

        let (block1_opt, block2_opt) = try_join!(block1_fut, block2_fut)?;

        match (block1_opt, block2_opt) {
            (Some(b1), Some(b2)) => Ok(Some((b1, b2))),
            _ => Ok(None),
        }
    }

    /// (Default) Retrieves a specific range of transactions from a block
    /// identified by its hash.
    ///
    /// # Arguments
    ///
    /// * `block_hash_hex`: The hex-encoded hash of the block.
    /// * `start_index`: The starting index (0-based) of the transaction range.
    /// * `count`: The maximum number of transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<TransactionResponse>>)`: Contains the transactions in
    ///   the specified range if the block and range are valid. Returns `None`
    ///   if the block itself is not found.
    /// * `Err(DbError)`: If a database error occurs.
    async fn get_block_transaction_range_by_hash(
        &self,
        block_hash_hex: &str,
        start_index: usize,
        count: usize,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        if let Some(all_txs) =
            self.get_block_transactions_by_hash(block_hash_hex).await?
        {
            let range_txs: Vec<_> =
                all_txs.into_iter().skip(start_index).take(count).collect();
            Ok(Some(range_txs))
        } else {
            Ok(None)
        }
    }

    /// (Default) Retrieves the last `count` transactions from a block
    /// identified by its height.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the block.
    /// * `count`: The maximum number of last transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<TransactionResponse>>)`: Contains the last `count`
    ///   transactions if the block is found. Returns `None` if the block itself
    ///   is not found.
    /// * `Err(DbError)`: If a database error occurs.
    async fn get_last_block_transactions_by_height(
        &self,
        height: u64,
        count: usize,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        if let Some(all_txs) =
            self.get_block_transactions_by_height(height).await?
        {
            let start_index = all_txs.len().saturating_sub(count);
            let last_txs: Vec<_> =
                all_txs.into_iter().skip(start_index).collect();
            Ok(Some(last_txs))
        } else {
            Ok(None)
        }
    }

    /// (Default) Retrieves a specific range of transactions from a block
    /// identified by its height.
    ///
    /// # Arguments
    ///
    /// * `height`: The height of the block.
    /// * `start_index`: The starting index (0-based) of the transaction range.
    /// * `count`: The maximum number of transactions to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Vec<TransactionResponse>>)`: Contains the transactions in
    ///   the specified range if the block and range are valid. Returns `None`
    ///   if the block itself is not found.
    /// * `Err(DbError)`: If a database error occurs.
    async fn get_block_transaction_range_by_height(
        &self,
        height: u64,
        start_index: usize,
        count: usize,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        if let Some(all_txs) =
            self.get_block_transactions_by_height(height).await?
        {
            let range_txs: Vec<_> =
                all_txs.into_iter().skip(start_index).take(count).collect();
            Ok(Some(range_txs))
        } else {
            Ok(None)
        }
    }
}

// --- Concrete DatabaseAdapter Implementations ---

/// Concrete implementation of [`DatabaseAdapter`] that wraps the Rusk node's
/// live blockchain state database (`node::database::rocksdb::Backend`).
///
/// This adapter provides access to the current state of the blockchain,
/// mempool, and consensus data by interacting with the underlying RocksDB
/// database via the `node::database` traits (`Ledger`, `Mempool`,
/// `ConsensusStorage`, `Metadata`).
///
/// It implements the **required** primitive methods of the `DatabaseAdapter`
/// trait, delegating calls to the underlying database backend. For database
/// operations that might block the async runtime, this adapter uses
/// `tokio::task::spawn_blocking` to execute them on a separate thread pool.
///
/// ## Thread Safety and Blocking
///
/// - The underlying database backend (`node::database::rocksdb::Backend`) is
///   wrapped in an `Arc<tokio::sync::RwLock<...>>` to allow shared, thread-safe
///   access across multiple async tasks.
/// - To avoid lifetime issues when moving the database handle into
///   `spawn_blocking`, the `Arc` is cloned, and the lock (`RwLockReadGuard` or
///   `RwLockWriteGuard`) is acquired synchronously *within* the blocking task
///   using `blocking_read()` or `blocking_write()`.
///
/// ## Feature Flag
///
/// This implementation requires the `chain` feature flag to be enabled.
#[cfg(feature = "chain")]
#[derive(Clone)]
pub struct RuskDbAdapter {
    db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
}

#[cfg(feature = "chain")]
impl std::fmt::Debug for RuskDbAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuskDbAdapter")
            .field("db_client", &"<Rusk Database Client>")
            .finish_non_exhaustive() // Hide internal details
    }
}

#[cfg(feature = "chain")]
impl RuskDbAdapter {
    /// Creates a new `RuskDbAdapter` instance.
    ///
    /// # Arguments
    ///
    /// * `db_client` - A shared reference to the Rusk node's database client.
    ///
    /// # Returns
    ///
    /// A new `RuskDbAdapter` instance.
    pub fn new(
        db_client: Arc<tokio::sync::RwLock<node::database::rocksdb::Backend>>,
    ) -> Self {
        Self { db_client }
    }
}

#[cfg(feature = "chain")]
#[async_trait]
impl DatabaseAdapter for RuskDbAdapter {
    // --- Ledger Primitives ---

    async fn get_block_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::Block>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        let block_with_label_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        Ok(block_with_label_result.map(model::block::Block::from))
    }

    async fn get_block_transactions_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<Vec<model::transaction::TransactionResponse>>, DbError>
    {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        let block_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        match block_result {
            Some(block) => {
                let transactions = block
                    .txs()
                    .iter()
                    .cloned()
                    .map(model::transaction::TransactionResponse::from)
                    .collect();
                Ok(Some(transactions))
            }
            None => Ok(None),
        }
    }

    async fn get_block_faults_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockFaults>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        let block_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        match block_result {
            Some(block) => {
                let faults: Vec<ledger::Fault> = block.faults().to_vec();
                let block_faults = model::block::BlockFaults::try_from(faults)
                    .map_err(|e| {
                        DbError::InternalError(format!(
                            "Failed to convert faults: {}",
                            e
                        ))
                    })?;
                Ok(Some(block_faults))
            }
            None => Ok(None),
        }
    }

    async fn get_block_hash_by_height(
        &self,
        height: u64,
    ) -> Result<Option<String>, DbError> {
        let db_client = self.db_client.clone();
        let hash_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block_hash_by_height(height))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        Ok(hash_result.map(hex::encode))
    }

    async fn get_block_header_by_hash(
        &self,
        block_hash_hex: &str,
    ) -> Result<Option<model::block::BlockHeader>, DbError> {
        let block_hash: [u8; 32] = hex::decode(block_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid block hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid block hash length".into())
            })?;

        let db_client = self.db_client.clone();
        let header_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block_header(&block_hash[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        Ok(header_result.map(model::block::BlockHeader::from))
    }

    async fn get_block_label_by_height(
        &self,
        height: u64,
    ) -> Result<Option<model::block::BlockLabel>, DbError> {
        let db_client = self.db_client.clone();
        let label_result = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.block_label_by_height(height))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)?;

        Ok(label_result
            .map(|(_hash, label)| model::block::BlockLabel::from(label)))
    }

    async fn get_spent_transaction_by_hash(
        &self,
        tx_hash_hex: &str,
    ) -> Result<Option<model::transaction::TransactionInfo>, DbError> {
        let tx_hash: [u8; 32] = hex::decode(tx_hash_hex)
            .map_err(|e| {
                DbError::InternalError(format!("Invalid tx hash hex: {}", e))
            })?
            .try_into()
            .map_err(|_| {
                DbError::InternalError("Invalid tx hash length".into())
            })?;

        let db_client = self.db_client.clone();
        let spent_tx_opt = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.ledger_tx(&tx_hash[..]))
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })??;

        if let Some(spent_tx) = spent_tx_opt {
            let header_opt = self
                .get_block_header_by_height(spent_tx.block_height)
                .await?;

            let (block_hash, timestamp) = header_opt
                .map_or((String::new(), 0), |h| (h.hash, h.timestamp));

            let tx_index = None;

            let response = model::transaction::TransactionResponse::from(
                spent_tx.inner.clone(),
            );

            Ok(Some(model::transaction::TransactionInfo {
                base: response.base,
                transaction_data: response.transaction_data,
                block_height: spent_tx.block_height,
                block_hash,
                tx_index,
                gas_spent: spent_tx.gas_spent,
                timestamp,
                error: spent_tx.err,
            }))
        } else {
            Ok(None)
        }
    }

    async fn ledger_tx_exists(
        &self,
        tx_id: &[u8; 32],
    ) -> Result<bool, DbError> {
        let tx_id_copy = *tx_id;
        let db_client = self.db_client.clone();
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.ledger_tx_exists(&tx_id_copy[..]))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }

    // --- Mempool Primitives ---

    async fn mempool_tx(
        &self,
        tx_id: [u8; 32],
    ) -> Result<Option<model::transaction::TransactionResponse>, DbError> {
        let db_client = self.db_client.clone();
        let node_tx_opt = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_tx(tx_id))
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })??;

        Ok(node_tx_opt.map(model::transaction::TransactionResponse::from))
    }

    async fn mempool_tx_exists(
        &self,
        tx_id: [u8; 32],
    ) -> Result<bool, DbError> {
        let db_client = self.db_client.clone();
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_tx_exists(tx_id))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }

    async fn mempool_txs_sorted_by_fee(
        &self,
    ) -> Result<Vec<model::transaction::TransactionResponse>, DbError> {
        let db_client = self.db_client.clone();

        let join_result: Result<
            Vec<ledger::Transaction>,
            tokio::task::JoinError,
        > = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            // The closure passed to view returns Vec<Transaction>
            db.view(|v| {
                v.mempool_txs_sorted_by_fee()
                    .collect::<Vec<ledger::Transaction>>()
            }) // view itself returns Vec<Transaction>
        })
        .await;

        // Handle potential JoinError from spawn_blocking
        let collected_node_txs: Vec<ledger::Transaction> = join_result
            .map_err(|e| {
                DbError::InternalError(format!("Task join error: {}", e))
            })?;

        let response_txs = collected_node_txs
            .into_iter()
            .map(model::transaction::TransactionResponse::from)
            .collect();

        Ok(response_txs)
    }

    async fn mempool_txs_count(&self) -> Result<usize, DbError> {
        let db_client = self.db_client.clone();
        let count = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_txs_count())
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })?;
        Ok(count)
    }

    async fn mempool_txs_ids_sorted_by_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError> {
        let db_client = self.db_client.clone();
        let result_vec = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.mempool_txs_ids_sorted_by_fee().collect::<Vec<_>>())
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })?;
        Ok(result_vec)
    }

    async fn mempool_txs_ids_sorted_by_low_fee(
        &self,
    ) -> Result<Vec<(u64, [u8; 32])>, DbError> {
        let db_client = self.db_client.clone();
        let result_vec = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| {
                v.mempool_txs_ids_sorted_by_low_fee().collect::<Vec<_>>()
            })
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })?;
        Ok(result_vec)
    }

    // --- ConsensusStorage Primitives ---

    async fn candidate(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        let hash_copy = *hash;
        let db_client = self.db_client.clone();
        let node_block_opt = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.candidate(&hash_copy[..]))
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })??;
        Ok(node_block_opt.map(model::block::CandidateBlock::from))
    }

    async fn candidate_by_iteration(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::block::CandidateBlock>, DbError> {
        let header_copy = *header;
        let db_client = self.db_client.clone();
        let node_block_opt = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.candidate_by_iteration(&header_copy))
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })??;
        Ok(node_block_opt.map(model::block::CandidateBlock::from))
    }

    async fn validation_result(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<model::consensus::ValidationResult>, DbError> {
        let header_copy = *header;
        let db_client = self.db_client.clone();
        let node_result_opt = tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.validation_result(&header_copy))
        })
        .await
        .map_err(|e| {
            DbError::InternalError(format!("Task join error: {}", e))
        })??;
        Ok(node_result_opt.map(model::consensus::ValidationResult::from))
    }

    // --- Metadata Primitives ---

    async fn metadata_op_read(
        &self,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, DbError> {
        let key_copy = key.to_vec();
        let db_client = self.db_client.clone();
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_read();
            db.view(|v| v.op_read(&key_copy))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }

    async fn metadata_op_write(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), DbError> {
        let key_copy = key.to_vec();
        let value_copy = value.to_vec();
        let db_client = self.db_client.clone();
        tokio::task::spawn_blocking(move || {
            let db = db_client.blocking_write();
            db.update(|v| v.op_write(&key_copy, &value_copy))
        })
        .await
        .map_err(|e| DbError::InternalError(format!("Task join error: {}", e)))?
        .map_err(DbError::from)
    }
}
