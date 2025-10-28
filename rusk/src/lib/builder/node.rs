// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::time::Duration;

use kadcast::config::Config as KadcastConfig;
use node::chain::ChainSrv;
use node::database::rocksdb;
use node::database::{DatabaseOptions, DB};
use node::databroker::conf::Params as BrokerParam;
use node::databroker::DataBrokerSrv;
use node::mempool::conf::Params as MempoolParam;
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::telemetry::TelemetrySrv;
use node::{LongLivedService, Node};

use tokio::sync::{broadcast, mpsc};
use tracing::info;
#[cfg(feature = "archive")]
use {dusk_bytes::Serializable, node::archive::Archive, tracing::debug};

use crate::http::{DataSources, HttpServer, HttpServerConfig};
use crate::node::{
    ChainEventStreamer, DriverStore, RuskNode, RuskVmConfig, Services,
};
use crate::{Rusk, VERSION};

#[derive(Default)]
pub struct RuskNodeBuilder {
    consensus_keys_path: String,
    databroker: BrokerParam,
    kadcast: KadcastConfig,
    mempool: MempoolParam,
    telemetry_address: Option<String>,
    db_path: PathBuf,
    db_options: DatabaseOptions,
    max_chain_queue_size: usize,
    genesis_timestamp: u64,
    vm_config: RuskVmConfig,
    min_gas_limit: Option<u64>,
    feeder_call_gas: u64,
    state_dir: PathBuf,

    http: Option<HttpServerConfig>,

    driver_store_path: PathBuf,

    command_revert: bool,
    blob_expire_after: Option<u64>,
}

#[cfg(not(feature = "archive"))]
/// The default blob expiration period in blocks, equivalent to at least 10
/// days: max 6 blocks per min * 60 * 24 * 10
pub const DEFAULT_BLOB_EXPIRE_AFTER: u64 = 86_400u64;

#[cfg(feature = "archive")]
/// The default blob expiration period in blocks for archive nodes is 0, meaning
/// that blobs never expire
pub const DEFAULT_BLOB_EXPIRE_AFTER: u64 = 0;

const DEFAULT_MIN_GAS_LIMIT: u64 = 75000;
impl RuskNodeBuilder {
    pub fn with_consensus_keys(mut self, consensus_keys_path: String) -> Self {
        self.consensus_keys_path = consensus_keys_path;
        self
    }

    pub fn with_databroker<P: Into<BrokerParam>>(
        mut self,
        databroker: P,
    ) -> Self {
        self.databroker = databroker.into();
        self
    }

    pub fn with_kadcast<K: Into<kadcast::config::Config>>(
        mut self,
        kadcast: K,
    ) -> Self {
        self.kadcast = kadcast.into();
        self.kadcast.version = VERSION.to_string();
        self
    }

    pub fn with_db_path(mut self, db_path: PathBuf) -> Self {
        self.db_path = db_path;
        self
    }

    pub fn with_db_options(mut self, db_options: DatabaseOptions) -> Self {
        self.db_options = db_options;
        self
    }

    pub fn with_telemetry(
        mut self,
        telemetry_listen_add: Option<String>,
    ) -> Self {
        self.telemetry_address = telemetry_listen_add;
        self
    }

    pub fn with_mempool(mut self, conf: MempoolParam) -> Self {
        self.mempool = conf;
        self
    }

    pub fn with_chain_queue_size(mut self, max_queue_size: usize) -> Self {
        self.max_chain_queue_size = max_queue_size;
        self
    }

    pub fn with_genesis_timestamp(mut self, genesis_timestamp: u64) -> Self {
        self.genesis_timestamp = genesis_timestamp;
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_generation_timeout<O: Into<Option<Duration>>>(
        mut self,
        generation_timeout: O,
    ) -> Self {
        self.vm_config.generation_timeout = generation_timeout.into();
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_gas_per_deploy_byte<O: Into<Option<u64>>>(
        mut self,
        gas_per_deploy_byte: O,
    ) -> Self {
        if let Some(gas_per_deploy_byte) = gas_per_deploy_byte.into() {
            self.vm_config.gas_per_deploy_byte = Some(gas_per_deploy_byte);
        }
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_min_deployment_gas_price<O: Into<Option<u64>>>(
        mut self,
        min_deployment_gas_price: O,
    ) -> Self {
        if let Some(min_deploy_gas_price) = min_deployment_gas_price.into() {
            self.vm_config.min_deployment_gas_price =
                Some(min_deploy_gas_price);
        }
        self
    }

    pub fn with_min_gas_limit(mut self, min_gas_limit: Option<u64>) -> Self {
        self.min_gas_limit = min_gas_limit;
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_min_deploy_points<O: Into<Option<u64>>>(
        mut self,
        min_deploy_points: O,
    ) -> Self {
        if let Some(min_deploy_points) = min_deploy_points.into() {
            self.vm_config.min_deploy_points = Some(min_deploy_points);
        }
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_block_gas_limit<O: Into<Option<u64>>>(
        mut self,
        block_gas_limit: O,
    ) -> Self {
        if let Some(block_gas_limit) = block_gas_limit.into() {
            self.vm_config.block_gas_limit = Some(block_gas_limit);
        }
        self
    }

    pub fn with_feeder_call_gas(mut self, feeder_call_gas: u64) -> Self {
        self.feeder_call_gas = feeder_call_gas;
        self
    }

    pub fn with_state_dir(mut self, state_dir: PathBuf) -> Self {
        self.state_dir = state_dir;
        self
    }

    pub fn with_http(mut self, http: HttpServerConfig) -> Self {
        self.http = Some(http);
        self
    }

    pub fn with_driver_store_path(
        mut self,
        driver_store_path: PathBuf,
    ) -> Self {
        self.driver_store_path = driver_store_path;
        self
    }

    pub fn with_revert(mut self) -> Self {
        self.command_revert = true;
        self
    }

    pub fn with_vm_config(mut self, vm_config: RuskVmConfig) -> Self {
        self.vm_config = vm_config;
        self
    }

    pub fn with_blob_expire_after(
        mut self,
        blob_expire_after: Option<u64>,
    ) -> Self {
        self.blob_expire_after = blob_expire_after;
        self
    }

    /// Build the RuskNode and corresponding services
    pub async fn build_and_run(self) -> anyhow::Result<()> {
        let channel_cap = self
            .http
            .as_ref()
            .map(|h| h.ws_event_channel_cap)
            .unwrap_or(1);
        let (rues_sender, rues_receiver) = broadcast::channel(channel_cap);
        let (node_sender, node_receiver) = mpsc::channel(1000);

        #[cfg(feature = "archive")]
        let archive = Archive::create_or_open(self.db_path.clone()).await;

        let min_gas_limit = self.min_gas_limit.unwrap_or(DEFAULT_MIN_GAS_LIMIT);
        let finality_activation = self
            .vm_config
            .feature(crate::node::FEATURE_ABI_PUBLIC_SENDER)
            .unwrap_or(u64::MAX);

        let blob_expire_after =
            self.blob_expire_after.unwrap_or(DEFAULT_BLOB_EXPIRE_AFTER);

        let rusk = Rusk::new(
            self.state_dir,
            self.kadcast.kadcast_id.unwrap_or_default(),
            self.vm_config,
            min_gas_limit,
            self.feeder_call_gas,
            rues_sender.clone(),
            #[cfg(feature = "archive")]
            archive.clone(),
            DriverStore::new(Some(self.driver_store_path)),
        )
        .map_err(|e| anyhow::anyhow!("Cannot instantiate VM {e}"))?;
        info!("Rusk VM loaded");

        let node = {
            let db = rocksdb::Backend::create_or_open(
                self.db_path.clone(),
                self.db_options.clone(),
            );
            let net = Kadcast::new(self.kadcast)?;
            RuskNode::new(
                Node::new(net, db, rusk.clone()),
                #[cfg(feature = "archive")]
                archive.clone(),
            )
        };

        let mut chain_srv = ChainSrv::new(
            self.consensus_keys_path,
            self.max_chain_queue_size,
            node_sender.clone(),
            self.genesis_timestamp,
            *crate::DUSK_CONSENSUS_KEY,
            finality_activation,
            blob_expire_after,
            #[cfg(feature = "archive")]
            archive.clone(),
        );
        if self.command_revert {
            chain_srv
                .initialize(
                    node.inner().network(),
                    node.inner().database(),
                    node.inner().vm_handler(),
                )
                .await?;
            return chain_srv.revert_last_final().await;
        }

        let mut service_list: Vec<Box<Services>> = vec![
            Box::new(MempoolSrv::new(self.mempool, node_sender.clone())),
            Box::new(chain_srv),
            Box::new(DataBrokerSrv::new(self.databroker)),
            Box::new(TelemetrySrv::new(self.telemetry_address)),
        ];

        let mut _ws_server = None;
        if let Some(http) = self.http {
            info!("Configuring HTTP");

            service_list.push(Box::new(ChainEventStreamer {
                node_receiver,
                rues_sender,
            }));

            let mut handler = DataSources::default();
            handler.sources.push(Box::new(rusk.clone()));
            handler.sources.push(Box::new(node.clone()));

            #[cfg(feature = "prover")]
            handler.sources.push(Box::new(rusk_prover::LocalProver));

            let cert_and_key = match (http.cert, http.key) {
                (Some(cert), Some(key)) => Some((cert, key)),
                _ => None,
            };

            _ws_server = Some(
                HttpServer::bind(
                    handler,
                    rues_receiver,
                    http.ws_event_channel_cap,
                    http.address,
                    http.headers,
                    cert_and_key,
                )
                .await?,
            );
        }

        node.inner().initialize(&mut service_list).await?;

        #[cfg(feature = "archive")]
        {
            if archive.fetch_active_accounts().await? == 0 {
                let base_commit = None;
                let accounts = rusk.moonlight_accounts(base_commit);

                let accounts = accounts
                    .map_err(|e| {
                        anyhow::anyhow!("Cannot get moonlight accounts: {e}")
                    })?
                    .map(|(_, pk)| bs58::encode(pk.to_bytes()).into_string())
                    .collect::<std::collections::HashSet<_>>();

                debug!("Found {} Moonlight accounts", accounts.len());

                archive.update_active_accounts(accounts).await?;
            }
        }

        node.inner().spawn_all(service_list).await?;

        Ok(())
    }
}
