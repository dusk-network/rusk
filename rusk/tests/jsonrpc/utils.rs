// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utility functions for JSON-RPC integration tests.

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;

use node_data::ledger::{self as node_ledger};
use node_data::message::{payload as node_payload, ConsensusHeader};

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rusk::jsonrpc::config::{ConfigError, HttpServerConfig, JsonRpcConfig};
use rusk::jsonrpc::infrastructure::archive::ArchiveAdapter;
use rusk::jsonrpc::infrastructure::db::DatabaseAdapter;
use rusk::jsonrpc::infrastructure::error::NetworkError;
use rusk::jsonrpc::infrastructure::error::{ArchiveError, DbError};
use rusk::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
use rusk::jsonrpc::infrastructure::metrics::MetricsCollector;
use rusk::jsonrpc::infrastructure::network::NetworkAdapter;
use rusk::jsonrpc::infrastructure::state::AppState;
use rusk::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
use rusk::jsonrpc::infrastructure::{error::VmError, vm::VmAdapter};
use rusk::jsonrpc::model;
use tempfile::{tempdir, TempDir};

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use dusk_bytes::Serializable;

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
pub(crate) fn create_mock_block(
    height: u64,
    _hash_prefix: &str,
) -> model::block::Block {
    // Use a simple, deterministic hex hash based on height
    let hash_bytes = [height as u8; 32];
    let hash = hex::encode(hash_bytes);
    let prev_hash = hex::encode([(height.saturating_sub(1)) as u8; 32]);

    model::block::Block {
        header: model::block::BlockHeader {
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
        status: Some(model::block::BlockStatus::Final),
        transactions: None,
        faults: None,
        transactions_count: 0,
        block_reward: Some(5000),
        total_gas_limit: Some(50_000),
    }
}

/// Creates a mock `MoonlightEventGroup` for testing.
pub(crate) fn create_mock_moonlight_group(
    tx_hash_prefix: &str,
    block_height: u64,
) -> model::archive::MoonlightEventGroup {
    model::archive::MoonlightEventGroup {
        origin: format!("{}_{}", tx_hash_prefix, block_height),
        block_height,
        events: vec![], // Keep it simple for mock tests
    }
}

/// Helper to create a simple Moonlight Tx Response for testing.
pub(crate) fn create_mock_ml_tx_response(
    hash: &str,
) -> model::transaction::TransactionResponse {
    model::transaction::TransactionResponse {
        base: model::transaction::BaseTransaction {
            tx_hash: hash.into(),
            version: 1,
            tx_type: model::transaction::TransactionType::Moonlight,
            gas_price: 10,
            gas_limit: 1000,
            raw: format!("raw_{}", hash),
        },
        status: Some(model::transaction::TransactionStatus {
            status: model::transaction::TransactionStatusType::Executed,
            block_height: Some(101),
            block_hash: Some(format!("bh_{}", hash)),
            gas_spent: Some(800),
            timestamp: Some(54321),
            error: None,
        }),
        transaction_data: model::transaction::TransactionDataType::Moonlight(
            model::transaction::MoonlightTransactionData {
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
}

// --- Mock Archive Adapter ---

/// A mock implementation of `ArchiveAdapter` for testing purposes.
#[derive(Debug, Clone, Default)]
pub struct MockArchiveAdapter {
    /// Mock storage for transaction groups keyed by memo bytes (as Vec<u8>).
    pub txs_by_memo: HashMap<Vec<u8>, Vec<model::archive::MoonlightEventGroup>>,
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
    pub bootstrapping_nodes: Option<Vec<String>>,
    /// Predefined public address.
    pub public_address: Option<SocketAddr>,
    /// Predefined list of alive peers.
    pub alive_peers: Option<Vec<SocketAddr>>,
    /// Predefined count of alive peers.
    pub alive_peers_count: Option<usize>,
    /// Predefined list of peer locations.
    pub peer_locations: Option<Vec<model::network::PeerLocation>>,
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

    async fn get_bootstrapping_nodes(
        &self,
    ) -> Result<Vec<String>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.bootstrapping_nodes.clone().unwrap_or_else(|| {
            vec!["MockNetwork_1".to_string(), "MockNetwork_2".to_string()]
        }))
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

    async fn get_network_peers_location(
        &self,
    ) -> Result<Vec<model::network::PeerLocation>, NetworkError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.peer_locations.clone().unwrap_or_default())
    }
}

// --- Mock VM Adapter ---

/// Mock implementation of `VmAdapter` for testing.
#[derive(Default)]
pub struct MockVmAdapter {
    /// Force an error on all method calls if Some.
    pub force_error: Option<VmError>,
    /// Predefined simulation result.
    pub simulation_result: Option<model::transaction::SimulationResult>,
    /// Predefined preverification result.
    pub preverification_result: Option<model::vm::VmPreverificationResult>,
    /// Predefined list of provisioners with model types.
    pub provisioners: Option<
        Vec<(
            model::provisioner::ProvisionerKeys,
            model::provisioner::ProvisionerStakeData,
        )>,
    >,
    /// Predefined stake info map (BLS pubkey -> ConsensusStakeInfo).
    pub stakes: Option<HashMap<String, model::provisioner::ConsensusStakeInfo>>,
    /// Predefined state root.
    pub state_root: Option<[u8; 32]>,
    /// Predefined block gas limit.
    pub block_gas_limit: Option<u64>,
    /// Predefined chain ID.
    pub chain_id: Option<u8>,
    /// Predefined AccountData map for get_account_data. Use Vec as
    /// BlsPublicKey doesn't impl Ord or Hash.
    /// Stores model::account::AccountInfo now.
    pub account_data: Option<Vec<(BlsPublicKey, model::account::AccountInfo)>>,
    /// Predefined VmConfig
    pub vm_config: Option<model::vm::VmConfig>,
    /// Set of existing nullifiers for the mock.
    pub existing_nullifiers_set: Option<std::collections::HashSet<[u8; 32]>>,
}

// Manual implementation of Debug
impl std::fmt::Debug for MockVmAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockVmAdapter")
            .field("force_error", &self.force_error)
            .field("simulation_result", &self.simulation_result)
            .field("preverification_result", &self.preverification_result)
            .field("provisioners", &self.provisioners)
            .field("stakes", &self.stakes)
            .field("state_root", &self.state_root)
            .field("block_gas_limit", &self.block_gas_limit)
            .field("chain_id", &self.chain_id)
            .field("account_data", &self.account_data)
            .field("vm_config", &self.vm_config)
            .field("existing_nullifiers_set", &self.existing_nullifiers_set)
            .finish()
    }
}

// Manual implementation of Clone
impl Clone for MockVmAdapter {
    fn clone(&self) -> Self {
        Self {
            force_error: self.force_error.clone(),
            simulation_result: self.simulation_result.clone(),
            preverification_result: self.preverification_result.clone(),
            provisioners: self.provisioners.clone(),
            stakes: self.stakes.clone(),
            state_root: self.state_root,
            block_gas_limit: self.block_gas_limit,
            chain_id: self.chain_id,
            account_data: self.account_data.clone(),
            vm_config: self.vm_config.clone(),
            existing_nullifiers_set: self.existing_nullifiers_set.clone(),
        }
    }
}

#[async_trait::async_trait]
impl VmAdapter for MockVmAdapter {
    async fn simulate_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<model::transaction::SimulationResult, VmError> {
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
    ) -> Result<model::vm::VmPreverificationResult, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Return the predefined result or default to Valid
        Ok(self
            .preverification_result
            .clone()
            .unwrap_or(model::vm::VmPreverificationResult::Valid))
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
    ) -> Result<model::account::AccountInfo, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Iterate through Vec to find the key
        if let Some(vec) = &self.account_data {
            for (key, data) in vec {
                if key == pk {
                    return Ok(data.clone());
                }
            }
        }
        // Default if not found in vec or vec is None
        Ok(model::account::AccountInfo {
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
    ) -> Result<
        Vec<(
            model::provisioner::ProvisionerKeys,
            model::provisioner::ProvisionerStakeData,
        )>,
        VmError,
    > {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Return predefined or empty Vec
        Ok(self.provisioners.clone().unwrap_or_default())
    }

    async fn get_stake_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<model::provisioner::ConsensusStakeInfo>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Look up in the stakes map if provided
        if let Some(stakes_map) = &self.stakes {
            // Use Serializable::to_bytes()
            let pk_bytes = pk.to_bytes();
            let pk_b58 = bs58::encode(pk_bytes).into_string();
            Ok(stakes_map.get(&pk_b58).cloned())
        } else {
            Ok(None) // Default mock implementation: None
        }
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

    async fn get_vm_config(&self) -> Result<model::vm::VmConfig, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        let mut features = HashMap::with_capacity(1);
        features.insert("ABI_PUBLIC_SENDER".to_string(), 1000000);

        // Return a predefined config or a default config for the mock
        Ok(model::vm::VmConfig {
            block_gas_limit: 3000000000,
            gas_per_deploy_byte: 100,
            min_deploy_points: 5000000,
            min_deployment_gas_price: 2000,
            generation_timeout: Some(std::time::Duration::from_secs(2)),
            features,
        })
    }

    async fn validate_nullifiers(
        &self,
        nullifiers: &[[u8; 32]],
    ) -> Result<Vec<[u8; 32]>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }

        if let Some(existing_set) = &self.existing_nullifiers_set {
            let spent_nullifiers = nullifiers
                .iter()
                .filter(|n| existing_set.contains(*n))
                .cloned()
                .collect();
            Ok(spent_nullifiers)
        } else {
            // Default: no nullifiers exist if set is not provided
            Ok(Vec::new())
        }
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
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let archive =
        ::node::archive::Archive::create_or_open(temp_dir.path()).await;
    (temp_dir, archive)
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

// generate_tls_certs remains mostly the same, but returns HttpServerConfig
pub fn generate_tls_certs(
) -> Result<(TempDir, HttpServerConfig), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let cert_path = dir.path().join("cert.pem");
    let key_path = dir.path().join("key.pem");

    let mut params = CertificateParams::new(vec!["localhost".to_string()])?;
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "Rusk Test Cert");
    params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    // Use correct rcgen 0.13 methods with 'pem' feature
    let cert_pem = cert.pem(); // Get cert PEM string
    let key_pem = key_pair.serialize_pem(); // Serialize keypair to PEM string

    fs::write(&cert_path, cert_pem)?;
    fs::write(&key_path, key_pem)?;

    let http_config = HttpServerConfig {
        // Use a fixed, likely available port for testing instead of 0
        // If this port is taken, the test will fail, indicating need for a
        // different approach
        bind_address: "127.0.0.1:39989".parse()?,
        cert: Some(cert_path),
        key: Some(key_path),
        ..Default::default()
    };

    Ok((dir, http_config))
}

// Function to manually create AppState with custom JsonRpcConfig
pub fn create_custom_app_state(config: JsonRpcConfig) -> AppState {
    let db_mock = MockDbAdapter::default();
    let archive_mock = MockArchiveAdapter::default();
    let network_mock = MockNetworkAdapter::default();
    let vm_mock = MockVmAdapter::default();
    let sub_manager = SubscriptionManager::default();
    let metrics = MetricsCollector::default();
    let rate_limit_config = Arc::new(config.rate_limit.clone());
    let manual_rate_limiters = ManualRateLimiters::new(rate_limit_config)
        .expect("Failed to create manual rate limiters for custom config");

    AppState::new(
        config, // Use the provided config
        Arc::new(db_mock),
        Arc::new(archive_mock),
        Arc::new(network_mock),
        Arc::new(vm_mock),
        sub_manager,
        metrics,
        manual_rate_limiters,
    )
}
