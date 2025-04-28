// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utility functions for JSON-RPC integration tests.

use dusk_consensus::user::stake::Stake;

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::StakeData;
use dusk_core::stake::StakeKeys;
use dusk_core::transfer::moonlight::AccountData;

use node_data::ledger::{self as node_ledger};
use node_data::message::{payload as node_payload, ConsensusHeader};

use rusk::jsonrpc::infrastructure::error::NetworkError;
use rusk::jsonrpc::infrastructure::error::{ArchiveError, DbError};
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::network::NetworkAdapter;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use rusk::jsonrpc::infrastructure::{error::VmError, vm::VmAdapter};
use rusk::jsonrpc::model;
use rusk::jsonrpc::model::archive::MoonlightEventGroup;
use rusk::jsonrpc::model::block::{Block, BlockHeader, BlockStatus};
use rusk::jsonrpc::model::provisioner::StakeInfo;
use rusk::jsonrpc::model::transaction::{
    BaseTransaction, TransactionResponse, TransactionType,
};
use rusk::jsonrpc::{
    config::{ConfigError, JsonRpcConfig},
    model::transaction::SimulationResult,
};
use rusk::jsonrpc::{
    infrastructure::archive::ArchiveAdapter,
    model::provisioner::ProvisionerInfo,
};
use rusk::jsonrpc::{
    infrastructure::db::DatabaseAdapter,
    model::transaction::{
        MoonlightTransactionData, TransactionDataType, TransactionStatus,
        TransactionStatusType,
    },
};

use rusk::node::RuskVmConfig;

use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

// --- Helper Functions ---

/// Helper to get an ephemeral port by binding to port 0.
pub fn get_ephemeral_port() -> Result<std::net::SocketAddr, std::io::Error> {
    // Bind to port 0 to get an OS-assigned ephemeral port
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    // Drop the listener immediately to free the port for the actual server
    drop(listener);
    Ok(addr)
}

/// Creates a mock `Block` for testing with basic fields populated.
pub(crate) fn create_mock_block(height: u64, _hash_prefix: &str) -> Block {
    // Use a simple, deterministic hex hash based on height
    let hash_bytes = [height as u8; 32];
    let hash = hex::encode(hash_bytes);
    let prev_hash = hex::encode([(height.saturating_sub(1)) as u8; 32]);

    Block {
        header: BlockHeader {
            version: 1,
            height,
            previous_hash: prev_hash, // Deterministic prev hash
            timestamp: 1_600_000_000 + height * 1000,
            hash: hash.clone(), // Use deterministic hash
            state_hash: format!("state_{}", hash),
            validator: "validator_base58_key".to_string(),
            transactions_root: format!("txroot_{}", hash),
            gas_limit: 100_000,
            seed: format!("seed_{}", hash),
            sequence: 1,
        },
        status: Some(BlockStatus::Final),
        transactions: None,
        transactions_count: 0,
        block_reward: Some(5000),
        total_gas_limit: Some(50_000),
    }
}

/// Creates a mock `MoonlightEventGroup` for testing.
pub(crate) fn create_mock_moonlight_group(
    tx_hash_prefix: &str,
    block_height: u64,
) -> MoonlightEventGroup {
    MoonlightEventGroup {
        origin: format!("{}_{}", tx_hash_prefix, block_height),
        block_height,
        events: vec![], // Keep it simple for mock tests
    }
}

/// Helper to create a simple Moonlight Tx Response for testing.
pub(crate) fn create_mock_ml_tx_response(hash: &str) -> TransactionResponse {
    TransactionResponse {
        base: BaseTransaction {
            tx_hash: hash.into(),
            version: 1,
            tx_type: TransactionType::Moonlight,
            gas_price: 10,
            gas_limit: 1000,
            raw: format!("raw_{}", hash),
        },
        status: Some(TransactionStatus {
            status: TransactionStatusType::Executed,
            block_height: Some(101),
            block_hash: Some(format!("bh_{}", hash)),
            gas_spent: Some(800),
            timestamp: Some(54321),
            error: None,
        }),
        transaction_data: TransactionDataType::Moonlight(
            MoonlightTransactionData {
                sender: "sender".to_string(),
                receiver: Some("receiver".to_string()),
                value: 1000,
                nonce: 5,
                memo: Some("memo".to_string()),
            },
        ),
    }
}

#[allow(dead_code)]
pub(crate) fn assert_security_error<T>(
    result: &Result<T, ConfigError>,
    expected_substring: &str,
) {
    if let Err(e) = result {
        let error_string_lower = e.to_string().to_lowercase();
        let expected_substring_lower = expected_substring.to_lowercase();
        assert!(
            error_string_lower.contains(&expected_substring_lower),
            "Expected error message to contain (case-insensitive) '{}', but got: {}",
            expected_substring,
            e
        );
    } else {
        panic!(
            "Expected an error containing '{}', but got Ok",
            expected_substring
        );
    }
}

#[allow(dead_code)]
pub(crate) fn create_environment_config(
    _vars: &[(&str, &str)],
) -> JsonRpcConfig {
    JsonRpcConfig::default()
}

// --- Mock Database Adapter ---

/// A mock implementation of `DatabaseAdapter` for testing purposes.
/// Stores data in HashMaps.
#[derive(Debug, Clone, Default)]
pub struct MockDbAdapter {
    /// Mock storage for blocks keyed by height.
    pub blocks_by_height: HashMap<u64, Block>,
    /// Mock storage for blocks keyed by hex-encoded hash.
    pub blocks_by_hash: HashMap<String, Block>,
    /// Mock storage for headers keyed by height.
    pub headers_by_height: HashMap<u64, BlockHeader>,
    /// Mock storage for headers keyed by hex-encoded hash.
    pub headers_by_hash: HashMap<String, BlockHeader>,
    /// Mock storage for spent transactions keyed by hex-encoded hash.
    pub spent_txs_by_hash: HashMap<String, node_ledger::SpentTransaction>,
    /// Mock storage for mempool transactions keyed by tx_id.
    pub mempool_txs: HashMap<[u8; 32], node_ledger::Transaction>,
    /// Mock storage for candidate blocks keyed by block hash.
    pub candidates_by_hash: HashMap<[u8; 32], node_ledger::Block>,
    /// Mock storage for candidate blocks keyed by consensus header
    /// (serialized?). Using String representation for simplicity in mock.
    pub candidates_by_iteration: HashMap<String, node_ledger::Block>,
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
    ) -> Result<Option<Block>, DbError> {
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

    async fn get_block_label_by_height(
        &self,
        _height: u64,
    ) -> Result<Option<model::block::BlockLabel>, DbError> {
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
        tx_hash_hex: &str,
    ) -> Result<Option<node_ledger::SpentTransaction>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.spent_txs_by_hash.get(tx_hash_hex).cloned())
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
    ) -> Result<Option<node_ledger::Transaction>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.mempool_txs.get(&tx_id).cloned())
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
    ) -> Result<Vec<node_ledger::Transaction>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Mock: Sort stored mempool txs by gas_price (descending)
        let mut txs: Vec<_> = self.mempool_txs.values().cloned().collect();
        txs.sort_by_key(|b| std::cmp::Reverse(b.gas_price()));
        Ok(txs)
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
    ) -> Result<Option<node_ledger::Block>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self.candidates_by_hash.get(hash).cloned())
    }

    async fn candidate_by_iteration(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<node_ledger::Block>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Use a simple string representation for the key in the mock
        let key = format!(
            "{:?}-{}-{}",
            header.prev_block_hash, header.round, header.iteration
        );
        Ok(self.candidates_by_iteration.get(&key).cloned())
    }

    async fn validation_result(
        &self,
        header: &ConsensusHeader,
    ) -> Result<Option<node_payload::ValidationResult>, DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Use a simple string representation for the key in the mock
        let key = format!(
            "{:?}-{}-{}",
            header.prev_block_hash, header.round, header.iteration
        );
        Ok(self.validation_results.get(&key).cloned())
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
        &mut self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), DbError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        self.metadata.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
}

// --- Mock Archive Adapter ---

/// A mock implementation of `ArchiveAdapter` for testing purposes.
#[derive(Debug, Clone, Default)]
pub struct MockArchiveAdapter {
    /// Mock storage for transaction groups keyed by memo bytes (as Vec<u8>).
    pub txs_by_memo: HashMap<Vec<u8>, Vec<MoonlightEventGroup>>,
    /// Mock storage for last archived block (height, hash).
    pub last_archived_block: Option<(u64, String)>,
    /// Mock storage for events keyed by hex block hash.
    pub events_by_hash: HashMap<String, Vec<model::archive::ArchivedEvent>>,
    /// Mock storage for events keyed by block height.
    pub events_by_height: HashMap<u64, Vec<model::archive::ArchivedEvent>>,
    /// Mock storage for finalized events keyed by contract ID string.
    pub finalized_events_by_contract:
        HashMap<String, Vec<model::archive::ArchivedEvent>>,
    /// Mock mapping from input height to the next height with a phoenix tx.
    pub next_phoenix_height: HashMap<u64, Option<u64>>,
    /// Mock storage for moonlight history keyed by bs58 public key.
    pub moonlight_history:
        HashMap<String, Vec<model::archive::MoonlightEventGroup>>,
    /// Optional error to return from all methods.
    pub force_error: Option<ArchiveError>,
}

#[async_trait::async_trait]
impl ArchiveAdapter for MockArchiveAdapter {
    /// Returns predefined transaction groups based on memo, or `force_error` if
    /// set.
    async fn get_moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Get by Vec<u8> key
        Ok(self.txs_by_memo.get(&memo).cloned())
    }

    /// Returns the predefined `last_archived_block`, or `force_error` if set.
    async fn get_last_archived_block(
        &self,
    ) -> Result<(u64, String), ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Return Option<(u64, String)> or default
        self.last_archived_block.clone().ok_or_else(|| {
            ArchiveError::NotFound("Mock last archived block not set".into())
        })
    }

    /// Returns predefined events based on hash, or `force_error` if set.
    async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .events_by_hash
            .get(hex_block_hash)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns predefined events based on height, or `force_error` if set.
    async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .events_by_height
            .get(&block_height)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns events from the latest mock height, or `force_error` if set.
    async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        let (height, _) = self.get_last_archived_block().await?;
        self.get_block_events_by_height(height).await
    }

    /// Returns predefined finalized events based on contract ID, or
    /// `force_error` if set.
    async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .finalized_events_by_contract
            .get(contract_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns predefined next phoenix height, or `force_error` if set.
    async fn get_next_block_with_phoenix_transaction(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Return predefined Option<u64> or default None
        Ok(self
            .next_phoenix_height
            .get(&block_height)
            .cloned()
            .flatten())
    }

    /// Returns predefined moonlight history based on public key, or
    /// `force_error` if set.
    async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        _ord: Option<model::archive::Order>,
        _from_block: Option<u64>,
        _to_block: Option<u64>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Ignore order/range parameters in mock
        Ok(self.moonlight_history.get(&pk_bs58).cloned())
    }

    // --- Default Methods (Covered by Trait) ---
    // We rely on the default implementations provided in the trait definition.
    // get_last_archived_block_height is already implemented as a default
    // method.
}

// --- Mock Network Adapter ---

/// Mock implementation of `NetworkAdapter` for testing.
#[derive(Debug, Clone, Default)]
pub struct MockNetworkAdapter {
    /// Force an error on all method calls if Some.
    pub force_error: Option<NetworkError>,
    /// Predefined network info string.
    pub network_info: Option<String>,
    /// Predefined public address.
    pub public_address: Option<SocketAddr>,
    /// Predefined list of alive peers.
    pub alive_peers: Option<Vec<SocketAddr>>,
    /// Predefined count of alive peers.
    pub alive_peers_count: Option<usize>,
}

#[async_trait::async_trait]
impl NetworkAdapter for MockNetworkAdapter {
    async fn broadcast_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<(), NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Simple Ok for mock
        Ok(())
    }

    async fn get_network_info(&self) -> Result<String, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self
            .network_info
            .clone()
            .unwrap_or_else(|| "MockNetwork".to_string()))
    }

    async fn get_public_address(&self) -> Result<SocketAddr, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self
            .public_address
            .unwrap_or_else(|| ([127, 0, 0, 1], 9000).into()))
    }

    async fn get_alive_peers(
        &self,
        _max_peers: usize,
    ) -> Result<Vec<SocketAddr>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.alive_peers.clone().unwrap_or_default())
    }

    async fn get_alive_peers_count(&self) -> Result<usize, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.alive_peers_count.unwrap_or_default())
    }

    async fn flood_request(
        &self,
        _inv: node_data::message::payload::Inv,
        _ttl_seconds: Option<u64>,
        _hops: u16,
    ) -> Result<(), NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Simple Ok for mock
        Ok(())
    }
}

// --- Mock VM Adapter ---

/// Mock implementation of `VmAdapter` for testing.
#[derive(Default)]
pub struct MockVmAdapter {
    /// Force an error on all method calls if Some.
    pub force_error: Option<VmError>,
    /// Predefined simulation result.
    pub simulation_result: Option<SimulationResult>,
    /// Predefined list of provisioners.
    pub provisioners: Vec<ProvisionerInfo>,
    /// Predefined stake info map (BLS pubkey hex -> StakeInfo).
    pub stakes: HashMap<String, StakeInfo>,
    /// Predefined state root.
    pub state_root: Option<[u8; 32]>,
    /// Predefined block gas limit.
    pub block_gas_limit: Option<u64>,
    /// Predefined chain ID.
    pub chain_id: Option<u8>,
    /// Predefined AccountData map for get_account_data. Use Vec as
    /// BlsPublicKey doesn't impl Ord or Hash.
    pub account_data: Option<Vec<(BlsPublicKey, AccountData)>>,
}

// Manual implementation of Debug
impl std::fmt::Debug for MockVmAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockVmAdapter")
            .field("force_error", &self.force_error)
            .field("simulation_result", &self.simulation_result)
            .field("provisioners", &self.provisioners)
            .field("stakes", &self.stakes)
            .field("state_root", &self.state_root)
            .field("block_gas_limit", &self.block_gas_limit)
            .field("chain_id", &self.chain_id)
            .field("account_data", &self.account_data)
            .finish()
    }
}

// Manual implementation of Clone
impl Clone for MockVmAdapter {
    fn clone(&self) -> Self {
        Self {
            force_error: self.force_error.clone(),
            simulation_result: self.simulation_result.clone(),
            provisioners: self.provisioners.clone(),
            stakes: self.stakes.clone(),
            state_root: self.state_root,
            block_gas_limit: self.block_gas_limit,
            chain_id: self.chain_id,
            account_data: self.account_data.clone(),
        }
    }
}

#[async_trait::async_trait]
impl VmAdapter for MockVmAdapter {
    async fn simulate_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<SimulationResult, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        self.simulation_result.clone().ok_or_else(|| {
            VmError::InternalError("Mock simulation result not set".to_string())
        })
    }

    async fn preverify_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<::node::vm::PreverificationResult, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(::node::vm::PreverificationResult::Valid)
    }

    async fn get_chain_id(&self) -> Result<u8, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.chain_id.unwrap_or(0)) // Default mock value
    }

    async fn get_account_data(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Iterate through Vec to find the key
        if let Some(vec) = &self.account_data {
            for (key, data) in vec {
                if key == pk {
                    // BlsPublicKey implements PartialEq
                    return Ok(data.clone());
                }
            }
        }
        // Default if not found in vec or vec is None
        Ok(AccountData {
            balance: 0,
            nonce: 0,
        })
    }

    async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.state_root.unwrap_or([0u8; 32])) // Default mock value
    }

    async fn get_block_gas_limit(&self) -> Result<u64, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.block_gas_limit.unwrap_or(1_000_000_000)) // Default high limit
    }

    async fn get_provisioners(
        &self,
    ) -> Result<Vec<(StakeKeys, StakeData)>, VmError> {
        // Return an empty Vec for the mock, matching the trait signature
        Ok(Vec::new())
    }

    async fn get_stake_info_by_pk(
        &self,
        _pk: &BlsPublicKey,
    ) -> Result<Option<Stake>, VmError> {
        Ok(None) // Default mock implementation
    }

    async fn query_contract_raw(
        &self,
        _contract_id: dusk_core::abi::ContractId,
        _method: String,
        _base_commit: [u8; 32],
        _args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(Vec::new()) // Default mock: empty result
    }

    async fn get_vm_config(&self) -> Result<RuskVmConfig, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Return a default config for the mock
        Ok(RuskVmConfig::default())
    }
}

// --- Test AppState Creator ---

/// Creates a default AppState instance for use in tests, including
/// ManualRateLimiters.
///
/// Panics if `ManualRateLimiters` cannot be created from the config's rate
/// limits.
///
/// Allows specifying a custom bind address for the HTTP server, overriding
/// defaults and environment variables for test isolation.
pub(crate) fn create_test_app_state_with_addr(
    http_addr: Option<SocketAddr>,
) -> AppState {
    let mut config = JsonRpcConfig::default();
    if let Some(addr) = http_addr {
        config.http.bind_address = addr;
    }
    // Ensure the port is what we expect if None was passed (using default)
    else {
        assert_eq!(
            config.http.bind_address.port(),
            8546,
            "Default port assumption failed in create_test_app_state_with_addr"
        );
    }

    let db_mock = MockDbAdapter::default();
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters");

    // Create AppState using the potentially modified config
    AppState::new(
        config, // Use the (potentially modified) config
        Arc::new(db_mock),
        Arc::new(archive_mock),
        Arc::new(network_mock),
        Arc::new(vm_mock),
        sub_manager,
        metrics,
        manual_rate_limiters,
    )
}

// Keep the old helper for compatibility if needed, but point it to the new one
#[allow(dead_code)]
pub(crate) fn create_test_app_state() -> AppState {
    create_test_app_state_with_addr(None)
}

/// Helper to setup a basic `AppState` with mock adapters for testing.
pub fn setup_mock_app_state() -> (
    AppState,
    MockDbAdapter,
    MockArchiveAdapter,
    MockNetworkAdapter,
    MockVmAdapter,
) {
    let config = JsonRpcConfig::test_config();
    let db_mock = MockDbAdapter::default();
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters");

    let app_state = AppState::new(
        config,
        Arc::new(db_mock.clone()),
        Arc::new(archive_mock.clone()),
        Arc::new(network_mock.clone()),
        Arc::new(vm_mock.clone()),
        sub_manager,
        metrics,
        manual_rate_limiters,
    );

    (app_state, db_mock, archive_mock, network_mock, vm_mock)
}

/// Helper to setup a temporary archive
#[cfg(feature = "archive")]
pub(crate) async fn setup_test_archive(
) -> (tempfile::TempDir, ::node::archive::Archive) {
    use tempfile::tempdir;

    let temp_dir = tempdir().expect("Failed to create temp dir");
    let archive =
        ::node::archive::Archive::create_or_open(temp_dir.path()).await;
    (temp_dir, archive)
}

/// Helper to setup a temporary RocksDB database
#[cfg(feature = "chain")]
pub(crate) fn setup_test_db(
) -> (tempfile::TempDir, ::node::database::rocksdb::Backend) {
    use tempfile::tempdir;

    let temp_dir = tempdir().expect("Failed to create temp dir for DB");
    let db_opts = ::node::database::DatabaseOptions {
        create_if_missing: true, // Ensure DB is created
        ..Default::default()
    };
    // Call create_or_open via the DB trait
    let db = ::node::database::DB::create_or_open(temp_dir.path(), db_opts);
    (temp_dir, db)
}
