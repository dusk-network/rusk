// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod geo;
pub mod graphql;

use std::collections::HashMap;
use std::sync::Arc;

use dusk_bytes::DeserializableSlice;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::data::{BlobData, BlobSidecar};
use dusk_core::transfer::Transaction as ProtocolTransaction;
use dusk_vm::execute;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{self, Ledger, LightBlock, Mempool, Metadata, DB};
use node::mempool::MempoolSrv;
use node::vm::VMExecution;
use node_data::ledger::{SpendingId, Transaction};

use async_graphql::{
    EmptyMutation, EmptySubscription, Name, Schema, Variables,
};
use graphql::Query;
use serde_json::{json, Map, Value};
use tracing::error;

use super::event::RequestData;
use super::*;
use crate::node::RuskNode;
use crate::{VERSION, VERSION_BUILD};

const GQL_VAR_PREFIX: &str = "rusk-gqlvar-";

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
impl HandleRequest for RuskNode {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("graphql", _, "query") => true,
            ("transactions", _, "preverify") => true,
            ("transactions", _, "propagate") => true,
            ("transactions", _, "simulate") => true,
            ("network", _, "peers") => true,
            ("network", _, "peers_location") => true,
            ("node", _, "info") => true,
            ("account", Some(_), "status") => true,
            ("blocks", _, "gas-price") => true,
            ("blobs", Some(_), "commitment") => true,
            ("blobs", Some(_), "hash") => true,

            _ => false,
        }
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData> {
        match request.uri.inner() {
            ("graphql", _, "query") => {
                self.handle_gql(&request.data, &request.headers).await
            }
            ("transactions", _, "preverify") => {
                self.handle_preverify(request.data.as_bytes()).await
            }
            ("transactions", _, "propagate") => {
                self.propagate_tx(request.data.as_bytes()).await
            }
            ("transactions", _, "simulate") => {
                self.simulate_tx(request.data.as_bytes()).await
            }
            ("network", _, "peers") => {
                let amount = request.data.as_string().trim().parse()?;
                self.alive_nodes(amount).await
            }

            ("network", _, "peers_location") => self.peers_location().await,
            ("node", _, "info") => self.get_info().await,
            ("account", Some(pk), "status") => self.get_account(pk).await,
            ("blocks", _, "gas-price") => {
                let max_transactions = request
                    .data
                    .as_string()
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX);
                self.get_gas_price(max_transactions).await
            }

            ("blobs", Some(commitment), "commitment") => {
                let commitment = hex::decode(commitment)
                    .map_err(|_| anyhow::anyhow!("Invalid hex"))?;
                let hash = BlobData::hash_from_commitment(&commitment);
                self.blob_by_hash(&hash, request.is_json()).await
            }
            ("blobs", Some(hash), "hash") => {
                let hash = hex::decode(hash)
                    .map_err(|_| anyhow::anyhow!("Invalid hex"))?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid hash length"))?;
                self.blob_by_hash(&hash, request.is_json()).await
            }
            _ => anyhow::bail!("Unsupported"),
        }
    }
}
impl RuskNode {
    async fn handle_gql(
        &self,
        data: &RequestData,
        headers: &serde_json::Map<String, Value>,
    ) -> anyhow::Result<ResponseData> {
        let gql_query = data.as_string();

        #[cfg(feature = "archive")]
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data((self.db(), self.archive()))
            .finish();
        #[cfg(not(feature = "archive"))]
        let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
            .data((self.db(), ()))
            .finish();

        if gql_query.trim().is_empty() {
            return Ok(ResponseData::new(schema.sdl()));
        }

        let variables = variables_from_headers(headers);
        let gql_query =
            async_graphql::Request::new(gql_query).variables(variables);

        let gql_res = schema.execute(gql_query).await;
        let async_graphql::Response { data, errors, .. } = gql_res;
        if !errors.is_empty() {
            return Err(anyhow::anyhow!(serde_json::to_value(errors)?));
        }
        let data = serde_json::to_value(&data)
            .map_err(|e| anyhow::anyhow!("Cannot parse response {e}"))?;
        Ok(ResponseData::new(data))
    }

    async fn handle_preverify(
        &self,
        data: &[u8],
    ) -> anyhow::Result<ResponseData> {
        let tx = dusk_core::transfer::Transaction::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?;
        let db = self.inner().database();
        let vm = self.inner().vm_handler();
        let tx = tx.into();

        MempoolSrv::check_tx(&db, &vm, &tx, true, usize::MAX)
            .await
            .map_err(|e| {
                error!("Tx {} not accepted: {e}", hex::encode(tx.id()));
                e
            })?;

        Ok(ResponseData::new(DataType::None))
    }

    async fn propagate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
        let tx: Transaction = ProtocolTransaction::from_slice(tx)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?
            .into();
        let tx_message = tx.into();

        let network = self.network();
        network.read().await.route_internal(tx_message);

        Ok(ResponseData::new(DataType::None))
    }

    async fn alive_nodes(&self, amount: usize) -> anyhow::Result<ResponseData> {
        let nodes = self.network().read().await.alive_nodes(amount).await;
        let nodes: Vec<_> = nodes.iter().map(|n| n.to_string()).collect();
        Ok(ResponseData::new(serde_json::to_value(nodes)?))
    }

    async fn get_info(&self) -> anyhow::Result<ResponseData> {
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
    ) -> anyhow::Result<ResponseData> {
        let gas_prices: Vec<u64> =
            self.db()
                .read()
                .await
                .view(|t| -> anyhow::Result<Vec<u64>> {
                    Ok(t.mempool_txs_ids_sorted_by_fee()
                        .take(max_transactions)
                        .map(|(gas_price, _)| gas_price)
                        .collect())
                })?;

        if gas_prices.is_empty() {
            let stats = serde_json::json!({ "average": 1, "max": 1, "median": 1, "min": 1 });
            return Ok(ResponseData::new(serde_json::to_value(stats)?));
        }

        let mean_gas_price = {
            let total: u64 = gas_prices.iter().sum();
            let count = gas_prices.len() as u64;
            // ceiling division to round up
            (total + count - 1) / count
        };

        let max_gas_price = *gas_prices.iter().max().unwrap();

        let median_gas_price = {
            let mid = gas_prices.len() / 2;
            if gas_prices.len() % 2 == 0 {
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

    async fn simulate_tx(&self, tx: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = ProtocolTransaction::from_slice(tx)
            .map_err(|e| anyhow::anyhow!("Invalid transaction: {e:?}"))?;
        let (config, mut session) = {
            let vm_handler = self.inner().vm_handler();
            let vm_handler = vm_handler.read().await;
            if tx.gas_limit() > vm_handler.get_block_gas_limit() {
                return Err(anyhow::anyhow!("Gas limit is too high."));
            }
            let tip = load_tip(&self.db())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load the tip: {e}"))?
                .ok_or_else(|| anyhow::anyhow!("Could not find the tip"))?;
            let height = tip.header.height;
            let config = vm_handler.vm_config.to_execution_config(height);
            let session = vm_handler
                .new_block_session(height, vm_handler.tip.read().current)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to initialize a session: {e}")
                })?;
            (config, session)
        };
        let receipt = execute(&mut session, &tx, &config);
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
    ) -> anyhow::Result<ResponseData> {
        let blob = self.db().read().await.view(|t| {
            t.blob_data_by_hash(hash)?.ok_or(anyhow::anyhow!(
                "Blob with versioned hash {} not found",
                hex::encode(hash)
            ))
        })?;
        let response = if as_json {
            let sidecar =
                BlobSidecar::from_buf(&mut &blob[..]).map_err(|e| {
                    anyhow::anyhow!("Failed to parse blob sidecar: {e:?}")
                })?;
            let json = serde_json::to_value(sidecar)?;
            ResponseData::new(json)
        } else {
            ResponseData::new(blob)
        };
        Ok(response)
    }

    async fn get_account(&self, pk_str: &str) -> anyhow::Result<ResponseData> {
        let pk = bs58::decode(pk_str)
            .into_vec()
            .map_err(|_| anyhow::anyhow!("Invalid bs58 account"))?;
        let pk = BlsPublicKey::from_slice(&pk)
            .map_err(|_| anyhow::anyhow!("Invalid bls account"))?;

        let db = self.inner().database();
        let vm = self.inner().vm_handler();

        let account = vm
            .read()
            .await
            .account(&pk)
            .map_err(|e| anyhow::anyhow!("Cannot query the state {e:?}"))?;

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
}

async fn load_tip<DB: database::DB>(
    db: &Arc<RwLock<DB>>,
) -> anyhow::Result<Option<LightBlock>> {
    db.read().await.view(|t| {
        anyhow::Ok(t.op_read(MD_HASH_KEY)?.and_then(|tip_hash| {
            t.light_block(&tip_hash[..])
                .expect("block to be found if metadata is set")
        }))
    })
}
