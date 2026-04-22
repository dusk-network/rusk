// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod geo;
pub mod graphql;

use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::{
    BatchRequest, BatchResponse, EmptyMutation, EmptySubscription, Name,
    Schema, Variables,
};
use async_trait::async_trait;
use dusk_bytes::DeserializableSlice;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::data::{BlobData, BlobSidecar};
use dusk_vm::execute;
use graphql::Query;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{self, DB, Ledger, LightBlock, Mempool, Metadata};
use node::mempool::{MempoolSrv, TxAcceptanceError};
use node::vm::VMExecution;
use node_data::ledger::{CanonicalTransaction, LedgerTransaction, SpendingId};
use serde_json::{Map, Value, json};
use tracing::{error, warn};

use super::rues::event::RequestData;
use super::*;
use crate::node::{RuskNode, forward_event_to_rues, set_vm_host_context};
use crate::{VERSION, VERSION_BUILD};

const GQL_VAR_PREFIX: &str = "rusk-gqlvar-";
type GraphqlSchema = Schema<Query, EmptyMutation, EmptySubscription>;
type ChainResult<T> = Result<T, ChainError>;

#[derive(Debug, thiserror::Error)]
enum ChainError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unsupported operation")]
    Unsupported,
    #[error("Database error: {0}")]
    Database(String),
    #[error("VM error: {0}")]
    Vm(String),
    #[error("{0}")]
    Internal(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl ChainError {
    fn invalid_input<T: AsRef<str>>(msg: T) -> Self {
        Self::InvalidInput(msg.as_ref().to_string())
    }

    fn not_found<T: AsRef<str>>(msg: T) -> Self {
        Self::NotFound(msg.as_ref().to_string())
    }

    fn database<T: AsRef<str>>(msg: T) -> Self {
        Self::Database(msg.as_ref().to_string())
    }

    fn vm<T: AsRef<str>>(msg: T) -> Self {
        Self::Vm(msg.as_ref().to_string())
    }

    fn internal<T: AsRef<str>>(msg: T) -> Self {
        Self::Internal(msg.as_ref().to_string())
    }
}

impl From<ChainError> for HttpError {
    fn from(value: ChainError) -> Self {
        match value {
            ChainError::InvalidInput(msg) => HttpError::invalid_input(msg),
            ChainError::NotFound(msg) => HttpError::not_found(msg),
            ChainError::Unsupported => HttpError::Unsupported,
            ChainError::Database(msg) => HttpError::database(msg),
            ChainError::Vm(msg) => HttpError::vm(msg),
            ChainError::Internal(msg) => HttpError::internal(msg),
            ChainError::Serialization(err) => HttpError::Serialization(err),
        }
    }
}

async fn preverify_tx(
    node: &RuskNode,
    data: &[u8],
) -> ChainResult<LedgerTransaction> {
    let tip_height = load_tip_height(node).await?;
    let tx = decode_ingress_tx(data, tip_height)?;
    let db = node.inner().database();
    let vm = node.inner().vm_handler();

    let tx_id = hex::encode(tx.id());
    let tx = MempoolSrv::check_canonical_tx_at_tip(
        &db,
        &vm,
        &tx,
        tip_height,
        usize::MAX,
    )
    .await
    .map_err(|e| map_check_tx_error(tx_id, e))?;

    Ok(tx)
}

async fn load_tip_height(node: &RuskNode) -> ChainResult<u64> {
    node.inner()
        .database()
        .read()
        .await
        .view(|db| db.latest_block())
        .map_err(|e| {
            ChainError::database(format!(
                "Failed to load the latest block for tx decoding: {e}"
            ))
        })
        .map(|tip| tip.header.height)
}

fn decode_ingress_tx(
    data: &[u8],
    tip_height: u64,
) -> ChainResult<CanonicalTransaction> {
    CanonicalTransaction::decode_for_ingress(data, tip_height.saturating_add(1))
        .map_err(|e| ChainError::invalid_input(format!("Data: {e:?}")))
}

async fn forward_rues_events(
    node: &RuskNode,
    events: Vec<node_data::events::Event>,
) {
    let vm = node.inner().vm_handler();
    let rues_sender = vm.read().await.event_sender.clone();
    for event in events {
        forward_event_to_rues(&rues_sender, event);
    }
}

fn map_check_tx_error(tx_id: String, error: TxAcceptanceError) -> ChainError {
    let err_msg = format!("Tx {tx_id} not accepted: {error}");
    match error {
        TxAcceptanceError::MaxTxnCountExceeded(_)
        | TxAcceptanceError::Generic(_) => {
            error!("{err_msg}");
            ChainError::internal(err_msg)
        }
        TxAcceptanceError::AlreadyExistsInMempool
        | TxAcceptanceError::AlreadyExistsInLedger
        | TxAcceptanceError::BlobMissingSidecar(_)
        | TxAcceptanceError::BlobEmpty
        | TxAcceptanceError::BlobTooMany(_)
        | TxAcceptanceError::BlobInvalid(_)
        | TxAcceptanceError::SpendIdExistsInMempool
        | TxAcceptanceError::VerificationFailed(_)
        | TxAcceptanceError::MissingIntermediateNonce(_)
        | TxAcceptanceError::MaxFutureNonceQueueExceeded(_)
        | TxAcceptanceError::MaxMoonlightFutureNoncePerAccountExceeded(_)
        | TxAcceptanceError::GasPriceTooLow(_)
        | TxAcceptanceError::GasLimitTooLow(_)
        | TxAcceptanceError::UnsupportedIngressFormat { .. }
        | TxAcceptanceError::TooLarge
        | TxAcceptanceError::MaxSizeExceeded(_) => {
            warn!("{err_msg}");
            ChainError::invalid_input(err_msg)
        }
    }
}

fn variables_from_headers(headers: &Map<String, Value>) -> Variables {
    let mut var = Variables::default();
    headers
        .iter()
        .filter_map(|(h, v)| {
            let h = h.to_lowercase();
            h.starts_with(GQL_VAR_PREFIX).then(|| {
                (h.replacen(GQL_VAR_PREFIX, "", 1), async_graphql::value!(v))
            })
        })
        .for_each(|(k, v)| {
            var.insert(Name::new(k), v);
        });

    var
}

#[async_trait]
impl ChainRequestHandler for RuskNode {
    async fn graphql_query(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        self.handle_gql(&request.data, &request.headers)
            .await
            .map_err(HttpError::from)
    }

    async fn transactions(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "preverify" => self.handle_preverify(request.data.as_bytes()).await,
            "propagate" => self.propagate_tx(request.data.as_bytes()).await,
            "simulate" => self.simulate_tx(request.data.as_bytes()).await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn network(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "peers" => {
                let amount =
                    request.data.as_string().trim().parse().map_err(|_| {
                        ChainError::invalid_input("invalid amount")
                    })?;
                self.alive_nodes(amount).await
            }
            "peers_location" => self.peers_location().await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn node(
        &self,
        topic: &str,
        _request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "info" => self.get_info().await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn account(
        &self,
        entity: &str,
        topic: &str,
        _request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "status" => self.get_account(entity).await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn contract(
        &self,
        entity: &str,
        topic: &str,
        _request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "status" => self.get_contract_balance(entity).await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn blocks(
        &self,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "gas-price" => {
                let max_transactions = request
                    .data
                    .as_string()
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX);
                self.get_gas_price(max_transactions).await
            }
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn blobs(
        &self,
        entity: &str,
        topic: &str,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "commitment" => {
                let commitment = hex::decode(entity).map_err(|_| {
                    ChainError::invalid_input("commitment not hex")
                })?;
                let hash = BlobData::hash_from_commitment(&commitment);
                self.blob_by_hash(&hash, request.is_json()).await
            }
            "hash" => {
                let hash = hex::decode(entity)
                    .map_err(|_| ChainError::invalid_input("hash not hex"))?
                    .try_into()
                    .map_err(|_| ChainError::invalid_input("hash length"))?;
                self.blob_by_hash(&hash, request.is_json()).await
            }
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }

    async fn stats(
        &self,
        topic: &str,
        _request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let response = match topic {
            "account_count" => self.get_account_count().await,
            "tx_count" => self.get_tx_count().await,
            _ => Err(ChainError::Unsupported),
        };

        response.map_err(HttpError::from)
    }
}

impl RuskNode {
    fn graphql_schema(&self) -> GraphqlSchema {
        #[cfg(feature = "archive")]
        let init = (self.db(), self.archive());

        #[cfg(not(feature = "archive"))]
        let init = (self.db(), ());

        Schema::build(Query, EmptyMutation, EmptySubscription)
            .data(init)
            .finish()
    }

    async fn handle_gql(
        &self,
        data: &RequestData,
        headers: &serde_json::Map<String, Value>,
    ) -> ChainResult<ResponseData> {
        let gql_query = data.as_string();

        let schema = self.graphql_schema();

        if gql_query.trim().is_empty() {
            return Ok(ResponseData::new(schema.sdl()));
        }

        let variables = variables_from_headers(headers);
        let gql_query =
            async_graphql::Request::new(gql_query).variables(variables);

        graphql_legacy_response(schema.execute(gql_query).await)
    }
    async fn handle_preverify(&self, data: &[u8]) -> ChainResult<ResponseData> {
        preverify_tx(self, data).await?;
        Ok(ResponseData::new(DataType::None))
    }

    async fn propagate_tx(&self, tx: &[u8]) -> ChainResult<ResponseData> {
        let tip_height = load_tip_height(self).await?;
        let tx = decode_ingress_tx(tx, tip_height)?;
        let db = self.inner().database();
        let vm = self.inner().vm_handler();

        match MempoolSrv::check_canonical_tx_at_tip(
            &db,
            &vm,
            &tx,
            tip_height,
            usize::MAX,
        )
        .await
        {
            Ok(tx) => {
                let network = self.network();
                network.read().await.route_internal(tx.into());
            }
            Err(TxAcceptanceError::MissingIntermediateNonce(_)) => {
                let tx_id = hex::encode(tx.id());
                let tx_message: LedgerTransaction = tx.clone().into();
                let tx_message = tx_message.into();
                let (events, result) = self
                    .future_nonce_retry_queue()
                    .enqueue_message_report(&tx_message)
                    .await;
                forward_rues_events(self, events).await;
                result.map_err(|e| map_check_tx_error(tx_id, e))?;
            }
            Err(e) => {
                return Err(map_check_tx_error(hex::encode(tx.id()), e));
            }
        }

        Ok(ResponseData::new(DataType::None))
    }

    async fn alive_nodes(&self, amount: usize) -> ChainResult<ResponseData> {
        let nodes = self.network().read().await.alive_nodes(amount).await;
        let nodes: Vec<_> = nodes.iter().map(|n| n.to_string()).collect();
        Ok(ResponseData::new(serde_json::to_value(nodes)?))
    }

    async fn get_info(&self) -> ChainResult<ResponseData> {
        let mut info: HashMap<&str, serde_json::Value> = HashMap::new();

        info.insert("version", VERSION.as_str().into());
        info.insert("version_build", VERSION_BUILD.as_str().into());

        let n_conf = self.network().read().await.conf().clone();
        info.insert("bootstrapping_nodes", n_conf.bootstrapping_nodes.into());
        info.insert("chain_id", n_conf.kadcast_id.into());
        info.insert("kadcast_address", n_conf.public_address.into());

        let vm_conf = self.inner().vm_handler().read().await.vm_config.clone();
        let vm_conf = serde_json::to_value(vm_conf).unwrap_or_default();
        info.insert("vm_config", vm_conf);

        Ok(ResponseData::new(serde_json::to_value(&info)?))
    }

    /// Calculates various statistics for gas prices of transactions in the
    /// mempool.
    ///
    /// It retrieves a specified number of transactions, sorted by descending
    /// gas price, and calculates the average, maximum, minimum and median
    /// prices. In the absence of transactions, will
    /// default to a gas price of 1.
    ///
    /// # Arguments
    /// * `max_transactions` - Maximum number of transactions to consider.
    ///
    /// # Returns
    /// A JSON object encapsulating the statistics, or an error if processing
    /// fails.
    async fn get_gas_price(
        &self,
        max_transactions: usize,
    ) -> ChainResult<ResponseData> {
        let gas_prices: Vec<u64> = self
            .db()
            .read()
            .await
            .view(|t| -> anyhow::Result<Vec<u64>> {
                Ok(t.mempool_txs_ids_sorted_by_fee()
                    .take(max_transactions)
                    .map(|(gas_price, _)| gas_price)
                    .collect())
            })
            .map_err(|e| ChainError::database(e.to_string()))?;

        if gas_prices.is_empty() {
            let stats = serde_json::json!({ "average": 1, "max": 1, "median": 1, "min": 1 });
            return Ok(ResponseData::new(stats));
        }

        let mean_gas_price = {
            let total: u64 = gas_prices.iter().sum();
            let count = gas_prices.len() as u64;
            // ceiling division to round up
            total.div_ceil(count)
        };

        let max_gas_price = *gas_prices.iter().max().unwrap();

        let median_gas_price = {
            let mid = gas_prices.len() / 2;
            if gas_prices.len().is_multiple_of(2) {
                (gas_prices[mid - 1] + gas_prices[mid]) / 2
            } else {
                gas_prices[mid]
            }
        };

        let min_gas_price = *gas_prices.iter().min().unwrap();

        let stats = serde_json::json!({
            "average": mean_gas_price,
            "max": max_gas_price,
            "median": median_gas_price,
            "min": min_gas_price
        });

        Ok(ResponseData::new(serde_json::to_value(stats)?))
    }

    async fn simulate_tx(&self, tx: &[u8]) -> ChainResult<ResponseData> {
        // Keep the guard alive across `execute()` below.
        let (config, mut session, _host_query_policy_guard, tx) = {
            let vm_handler = self.inner().vm_handler();
            let vm_handler = vm_handler.read().await;
            let tip = load_tip(&self.db())
                .await
                .map_err(|e| {
                    ChainError::database(format!("Failed to load the tip: {e}"))
                })?
                .ok_or_else(|| {
                    ChainError::database("Could not find the tip")
                })?;
            let height = tip.header.height.saturating_add(1);
            let tx = CanonicalTransaction::decode_for_ingress(tx, height)
                .map_err(|e| {
                    ChainError::invalid_input(format!(
                        "Invalid transaction: {e:?}"
                    ))
                })?;
            if tx.protocol().gas_limit() > vm_handler.get_block_gas_limit() {
                return Err(ChainError::invalid_input(
                    "Gas limit is too high.",
                ));
            }
            let config = vm_handler.vm_config.to_execution_config(height);
            let _host_query_policy_guard =
                set_vm_host_context(&vm_handler.vm_config, height);
            let session = vm_handler
                .new_block_session(height, vm_handler.tip.read().current)
                .map_err(|e| {
                    ChainError::vm(format!(
                        "Failed to initialize a session: {e}"
                    ))
                })?;
            // Return the guard so the host-query policy is restored only after
            // `execute()` completes.
            (config, session, _host_query_policy_guard, tx)
        };
        let receipt = execute(&mut session, tx.protocol(), &config);
        let resp = match receipt {
            Ok(receipt) => json!({
                "gas-spent": receipt.gas_spent,
                "error": receipt.data.err().map(|err| format!("{err:?}")),
            }),
            Err(err) => json!({
                "gas-spent": 0,
                "error": format!("{err:?}")
            }),
        };
        Ok(ResponseData::new(resp))
    }

    async fn blob_by_hash(
        &self,
        hash: &[u8; 32],
        as_json: bool,
    ) -> ChainResult<ResponseData> {
        let blob = self
            .db()
            .read()
            .await
            .view(|t| t.blob_data_by_hash(hash))
            .map_err(|e| ChainError::database(e.to_string()))?
            .ok_or(ChainError::not_found(format!(
                "Blob with versioned hash {} not found",
                hex::encode(hash)
            )))?;
        let response = if as_json {
            let sidecar =
                BlobSidecar::from_buf(&mut &blob[..]).map_err(|e| {
                    ChainError::internal(format!(
                        "Failed to parse blob sidecar: {e:?}"
                    ))
                })?;
            let json = serde_json::to_value(sidecar)?;
            ResponseData::new(json)
        } else {
            ResponseData::new(blob)
        };
        Ok(response)
    }

    async fn get_account(&self, pk_str: &str) -> ChainResult<ResponseData> {
        let pk = bs58::decode(pk_str)
            .into_vec()
            .map_err(|_| ChainError::invalid_input("Invalid bs58 account"))?;
        let pk = BlsPublicKey::from_slice(&pk)
            .map_err(|_| ChainError::invalid_input("Invalid bls account"))?;

        let db = self.inner().database();
        let vm = self.inner().vm_handler();

        let account = vm.read().await.account(&pk).map_err(|e| {
            ChainError::vm(format!("Cannot query the state: {e:?}"))
        })?;

        // Determine the next available nonce not already used in the mempool.
        // This ensures that any in-flight transactions using sequential nonces
        // are accounted for.
        // If the account has no transactions in the mempool, the next_nonce is
        // the same as the account's current nonce + 1.
        let next_nonce = db
            .read()
            .await
            .view(|t| {
                let mut next_nonce = account.nonce + 1;
                loop {
                    let id = SpendingId::AccountNonce(pk, next_nonce);
                    if t.mempool_txs_by_spendable_ids(&[id]).is_empty() {
                        break;
                    }
                    next_nonce += 1;
                }
                anyhow::Ok(next_nonce)
            })
            .unwrap_or_else(|e| {
                error!("Failed to check the mempool for account {pk_str}: {e}");
                account.nonce + 1
            });

        Ok(ResponseData::new(json!({
            "balance": account.balance,
            "nonce": account.nonce,
            "next_nonce": next_nonce,
        })))
    }

    /// Returns the current balance for a specific smart contract.
    /// The response is a JSON object:
    /// ```json
    /// { "balance": 123456 }
    /// ```
    ///
    /// # Parameters
    /// * `contract_id_hex` — Hex-encoded 32-byte `ContractId` of the target
    ///   contract (without a `0x` prefix).
    ///
    /// # Errors
    /// Returns an error if:
    /// * The contract ID is not valid hex or not 32 bytes long.
    /// * The VM query to the Transfer contract fails.
    async fn get_contract_balance(
        &self,
        contract_id_hex: &str,
    ) -> ChainResult<ResponseData> {
        let contract_id = ContractId::try_from(contract_id_hex.to_owned())
            .map_err(|e| {
                ChainError::invalid_input(format!("Invalid contract ID: {e}"))
            })?;

        // Query the VM via the Transfer contract
        let vm = self.inner().vm_handler();
        let balance =
            vm.read()
                .await
                .contract_balance(&contract_id)
                .map_err(|e| {
                    ChainError::vm(format!(
                        "Failed to query contract balance: {e:?}"
                    ))
                })?;

        Ok(ResponseData::new(serde_json::json!({
            "balance": balance
        })))
    }

    /// Returns the total number of active public accounts recorded in the
    /// archive node. The response is a JSON object:
    /// ```json
    /// { "public_accounts": 12345 }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the archive feature is not enabled.
    async fn get_account_count(&self) -> ChainResult<ResponseData> {
        #[cfg(feature = "archive")]
        {
            let count = self
                .archive()
                .fetch_active_accounts()
                .await
                .map_err(|e| ChainError::database(e.to_string()))?;
            let body = serde_json::json!({ "public_accounts": count });
            Ok(ResponseData::new(body))
        }

        #[cfg(not(feature = "archive"))]
        Err(ChainError::Unsupported)
    }

    /// Returns the total number of finalized transactions observed in the
    /// archive, split into `public`, `shielded` and `total. The response is
    /// a JSON object:
    /// ```json
    /// { "public": 123, "shielded": 456, "total": 579 }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the archive feature is not enabled.
    async fn get_tx_count(&self) -> ChainResult<ResponseData> {
        #[cfg(feature = "archive")]
        {
            let (moonlight, phoenix) = self
                .archive()
                .fetch_tx_count()
                .await
                .map_err(|e| ChainError::database(e.to_string()))?;
            let total = moonlight + phoenix;
            let body = serde_json::json!({
                "public": moonlight,
                "shielded": phoenix,
                "total": total
            });
            Ok(ResponseData::new(body))
        }

        #[cfg(not(feature = "archive"))]
        Err(ChainError::Unsupported)
    }
}

fn graphql_legacy_response(
    gql_res: async_graphql::Response,
) -> ChainResult<ResponseData> {
    let async_graphql::Response { data, errors, .. } = gql_res;
    if errors.is_empty() {
        let data = serde_json::to_value(&data)?;
        return Ok(ResponseData::new(data));
    }

    Ok(ResponseData::new(json!({
        "data": data,
        "errors": errors,
    })))
}

#[async_trait]
impl GraphqlHandler for RuskNode {
    async fn execute_graphql(&self, request: BatchRequest) -> BatchResponse {
        self.graphql_schema().execute_batch(request).await
    }
}

async fn load_tip<DB: database::DB>(
    db: &Arc<RwLock<DB>>,
) -> anyhow::Result<Option<LightBlock>> {
    db.read().await.view(|t| {
        let tip = match t.op_read(MD_HASH_KEY)? {
            Some(tip_hash) => t.light_block(&tip_hash[..])?,
            None => None,
        };
        anyhow::Ok(tip)
    })
}

#[cfg(test)]
mod tests {
    use async_graphql::{Response as GraphqlResponse, ServerError, Value};
    use dusk_core::transfer::TransactionFormat;
    use node::mempool::TxAcceptanceError;

    use super::{
        ChainError, DataType, HttpError, graphql_legacy_response,
        map_check_tx_error,
    };

    #[test]
    fn chain_error_variant_mapping_is_stable() {
        assert!(matches!(
            HttpError::from(ChainError::invalid_input("bad request")),
            HttpError::InvalidInput(_)
        ));
        assert!(matches!(
            HttpError::from(ChainError::Unsupported),
            HttpError::Unsupported
        ));
        assert!(matches!(
            HttpError::from(ChainError::database("db failure")),
            HttpError::Database(_)
        ));
        assert!(matches!(
            HttpError::from(ChainError::vm("vm failure")),
            HttpError::Vm(_)
        ));
        assert!(matches!(
            HttpError::from(ChainError::internal("internal failure")),
            HttpError::Internal(_)
        ));
    }

    #[test]
    fn preverify_generic_check_tx_error_maps_to_internal() {
        let err = map_check_tx_error(
            "deadbeef".to_string(),
            TxAcceptanceError::Generic(anyhow::anyhow!("boom")),
        );
        assert!(matches!(err, ChainError::Internal(_)));
    }

    #[test]
    fn preverify_validation_check_tx_error_maps_to_invalid_input() {
        let err = map_check_tx_error(
            "deadbeef".to_string(),
            TxAcceptanceError::GasPriceTooLow(1),
        );
        assert!(matches!(err, ChainError::InvalidInput(_)));
    }

    #[test]
    fn preverify_unsupported_ingress_format_maps_to_invalid_input() {
        let err = map_check_tx_error(
            "deadbeef".to_string(),
            TxAcceptanceError::UnsupportedIngressFormat {
                actual: TransactionFormat::PreAegis,
                minimum: TransactionFormat::Aegis,
            },
        );
        assert!(matches!(err, ChainError::InvalidInput(_)));
    }

    #[test]
    fn graphql_legacy_response_keeps_success_shape() {
        let response = graphql_legacy_response(GraphqlResponse::new(
            Value::from_json(serde_json::json!({ "ping": "pong" }))
                .expect("json should convert to GraphQL value"),
        ))
        .expect("graphql legacy response should serialize");

        assert_eq!(
            response.data(),
            &DataType::Json(serde_json::json!({ "ping": "pong" })),
        );
    }

    #[test]
    fn graphql_legacy_response_returns_graphql_errors_in_body() {
        let response =
            graphql_legacy_response(GraphqlResponse::from_errors(vec![
                ServerError::new("missing field", None),
            ]))
            .expect("graphql legacy response should serialize");

        let DataType::Json(body) = response.data() else {
            panic!("legacy GraphQL errors should serialize as JSON");
        };
        assert!(body["data"].is_null());
        assert_eq!(body["errors"][0]["message"], "missing field");
    }
}
