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
use {node::archive::Archive, node::archive::ArchivistSrv};

use crate::http::{DataSources, HttpServer, HttpServerConfig};
use crate::node::{ChainEventStreamer, RuskNode, Services};
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

    generation_timeout: Option<Duration>,
    gas_per_deploy_byte: Option<u64>,
    min_deployment_gas_price: Option<u64>,
    min_gas_limit: Option<u64>,
    min_deploy_points: Option<u64>,
    block_gas_limit: u64,
    feeder_call_gas: u64,
    state_dir: PathBuf,

    http: Option<HttpServerConfig>,

    command_revert: bool,
}

const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;
const DEFAULT_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2000;
const DEFAULT_MIN_GAS_LIMIT: u64 = 75000;
const DEFAULT_MIN_DEPLOY_POINTS: u64 = 5_000_000;

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

    pub fn with_generation_timeout(
        mut self,
        generation_timeout: Option<Duration>,
    ) -> Self {
        self.generation_timeout = generation_timeout;
        self
    }

    pub fn with_gas_per_deploy_byte(
        mut self,
        gas_per_deploy_byte: Option<u64>,
    ) -> Self {
        self.gas_per_deploy_byte = gas_per_deploy_byte;
        self
    }

    pub fn with_min_deployment_gas_price(
        mut self,
        min_deployment_gas_price: Option<u64>,
    ) -> Self {
        self.min_deployment_gas_price = min_deployment_gas_price;
        self
    }

    pub fn with_min_gas_limit(mut self, min_gas_limit: Option<u64>) -> Self {
        self.min_gas_limit = min_gas_limit;
        self
    }

    pub fn with_min_deploy_points(
        mut self,
        min_deploy_points: Option<u64>,
    ) -> Self {
        self.min_deploy_points = min_deploy_points;
        self
    }

    pub fn with_block_gas_limit(mut self, block_gas_limit: u64) -> Self {
        self.block_gas_limit = block_gas_limit;
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

    pub fn with_revert(mut self) -> Self {
        self.command_revert = true;
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
        let (archive_sender, archive_receiver) = mpsc::channel(1000);

        let gas_per_deploy_byte = self
            .gas_per_deploy_byte
            .unwrap_or(DEFAULT_GAS_PER_DEPLOY_BYTE);
        let min_deployment_gas_price = self
            .min_deployment_gas_price
            .unwrap_or(DEFAULT_MIN_DEPLOYMENT_GAS_PRICE);
        let min_gas_limit = self.min_gas_limit.unwrap_or(DEFAULT_MIN_GAS_LIMIT);
        let min_deploy_points =
            self.min_deploy_points.unwrap_or(DEFAULT_MIN_DEPLOY_POINTS);

        let rusk = Rusk::new(
            self.state_dir,
            self.kadcast.kadcast_id.unwrap_or_default(),
            self.generation_timeout,
            gas_per_deploy_byte,
            min_deployment_gas_price,
            min_gas_limit,
            min_deploy_points,
            self.block_gas_limit,
            self.feeder_call_gas,
            rues_sender.clone(),
            #[cfg(feature = "archive")]
            archive_sender.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Cannot instantiate VM {e}"))?;
        info!("Rusk VM loaded");

        #[cfg(feature = "archive")]
        let archive = Archive::create_or_open(self.db_path.clone()).await;

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
                #[cfg(feature = "archive")]
                archivist_sender: archive_sender,
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

        #[cfg(feature = "archive")]
        service_list.push(Box::new(ArchivistSrv {
            archive_receiver,
            archivist: archive,
        }));

        node.inner().initialize(&mut service_list).await?;
        node.inner().spawn_all(service_list).await?;

        Ok(())
    }
}
