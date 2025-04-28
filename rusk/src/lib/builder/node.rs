// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::sync::Arc;
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
use tracing::{error, info, warn};
#[cfg(feature = "archive")]
use {dusk_bytes::Serializable, node::archive::Archive, tracing::debug};

use crate::http::{DataSources, HttpServer, HttpServerConfig};
use crate::jsonrpc::config::JsonRpcConfig;
use crate::node::{ChainEventStreamer, RuskNode, RuskVmConfig, Services};
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

    command_revert: bool,
}

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
            self.vm_config.gas_per_deploy_byte = gas_per_deploy_byte;
        }
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_min_deployment_gas_price<O: Into<Option<u64>>>(
        mut self,
        min_deployment_gas_price: O,
    ) -> Self {
        if let Some(min_deploy_gas_price) = min_deployment_gas_price.into() {
            self.vm_config.min_deployment_gas_price = min_deploy_gas_price;
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
            self.vm_config.min_deploy_points = min_deploy_points;
        }
        self
    }

    #[deprecated(since = "1.0.3", note = "please use `with_vm_config` instead")]
    pub fn with_block_gas_limit<O: Into<Option<u64>>>(
        mut self,
        block_gas_limit: O,
    ) -> Self {
        if let Some(block_gas_limit) = block_gas_limit.into() {
            self.vm_config.block_gas_limit = block_gas_limit;
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

    pub fn with_revert(mut self) -> Self {
        self.command_revert = true;
        self
    }

    pub fn with_vm_config(mut self, vm_config: RuskVmConfig) -> Self {
        self.vm_config = vm_config;
        self
    }

    /// Build the RuskNode and corresponding services
    pub async fn build_and_run(self) -> anyhow::Result<()> {
        // --- Load JSON-RPC Configuration --- START ---
        let jsonrpc_config = match JsonRpcConfig::load_default() {
            Ok(config) => {
                info!("Successfully loaded JSON-RPC configuration.");
                // Check if the server should be enabled based on config
                // (e.g., using http.bind_address port or a specific
                // http.enabled flag if added later)
                // For now, assume port 0 means disabled, otherwise enabled.
                // TODO: Add an explicit `enabled` flag to HttpServerConfig?
                if config.http.bind_address.port() != 0 {
                    info!("JSON-RPC server is configured to be enabled.");
                    Some(config)
                } else {
                    warn!(
                        "JSON-RPC server is configured with port 0, disabling."
                    );
                    None
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to load JSON-RPC configuration, disabling server.");
                None // Server disabled if config fails to load
            }
        };
        // --- Load JSON-RPC Configuration --- END ---

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

        let rusk = Rusk::new(
            self.state_dir,
            self.kadcast.kadcast_id.unwrap_or_default(),
            self.vm_config,
            min_gas_limit,
            self.feeder_call_gas,
            rues_sender.clone(),
            #[cfg(feature = "archive")]
            archive.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Cannot instantiate VM {e}"))?;
        info!("Rusk VM loaded");

        // Rusk instance (type Rusk)
        let rusk = rusk;

        let node = {
            let db = rocksdb::Backend::create_or_open(
                self.db_path.clone(),
                self.db_options.clone(),
            );
            let net = Kadcast::new(self.kadcast)?;
            // Pass the owned Rusk instance to Node::new
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
            // RuskNode implements HandleRequest and is Clone
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

        // --- Conditionally Initialize JSON-RPC Components --- START ---
        if let Some(config) = jsonrpc_config {
            info!("JSON-RPC server enabled. Creating Adapters and AppState...");

            // - Get Component Handles
            #[cfg(feature = "chain")]
            let db_handle = node.db().clone(); // Arc<RwLock<Backend>>
            #[cfg(feature = "archive")]
            let archive_handle = node.archive().clone(); // This gets an owned Archive
            #[cfg(feature = "chain")]
            let network_handle = node.network().clone(); // Arc<Kadcast>
            #[cfg(feature = "chain")]
            let _vm_handle = node.inner().vm_handler(); // Access VM handler via inner

            // - Create RuskDbAdapter
            #[cfg(feature = "chain")]
            let db_adapter: Arc<
                dyn crate::jsonrpc::infrastructure::db::DatabaseAdapter,
            > = {
                use crate::jsonrpc::infrastructure::db::RuskDbAdapter;
                info!("Creating RuskDbAdapter...");
                let adapter = Arc::new(RuskDbAdapter::new(db_handle));
                info!("RuskDbAdapter created.");
                adapter
            };

            // - Create RuskArchiveAdapter
            #[cfg(feature = "archive")]
            let archive_adapter: Arc<
                dyn crate::jsonrpc::infrastructure::archive::ArchiveAdapter,
            > = {
                use crate::jsonrpc::infrastructure::archive::RuskArchiveAdapter;
                info!("Creating RuskArchiveAdapter...");
                // Wrap the owned Archive in Arc before passing to the
                // constructor
                let adapter_instance =
                    RuskArchiveAdapter::new(Arc::new(archive_handle));
                // Wrap the adapter instance in Arc for AppState
                let adapter = Arc::new(adapter_instance);
                info!("RuskArchiveAdapter created.");
                adapter
            };

            // - Create RuskNetworkAdapter
            #[cfg(feature = "chain")]
            let network_adapter = {
                use crate::jsonrpc::infrastructure::network::RuskNetworkAdapter;
                info!("Creating RuskNetworkAdapter...");
                // Pass the Arc<RwLock<Kadcast>> handle
                let adapter = Arc::new(RuskNetworkAdapter::new(network_handle));
                info!("RuskNetworkAdapter created.");
                adapter
            };

            // - Create RuskVmAdapter
            #[cfg(feature = "chain")]
            let vm_adapter = {
                use crate::jsonrpc::infrastructure::vm::RuskVmAdapter;
                info!("Creating RuskVmAdapter...");
                // Get the Arc<RwLock<Rusk>> handle
                let vm_handle = node.inner().vm_handler();
                // Pass it to the updated constructor
                let adapter = Arc::new(RuskVmAdapter::new(vm_handle));
                info!("RuskVmAdapter created.");
                adapter
            };

            // - Create Other AppState Components (Subs, Metrics, RateLimiters)
            info!("Creating other AppState components...");

            // Subscription Manager
            use crate::jsonrpc::infrastructure::subscription::manager::SubscriptionManager;
            let subscription_manager = SubscriptionManager::default(); // Directly use, as it likely needs Arc internally or AppState wraps
                                                                       // it
            info!("SubscriptionManager created.");

            // Metrics Collector
            use crate::jsonrpc::infrastructure::metrics::MetricsCollector;
            let metrics_collector = MetricsCollector::default(); // Directly use, as it likely needs Arc internally or AppState wraps
                                                                 // it
            info!("MetricsCollector created.");

            // Rate Limiters
            use crate::jsonrpc::infrastructure::manual_limiter::ManualRateLimiters;
            let rate_limit_config_arc = Arc::new(config.rate_limit.clone());
            let manual_rate_limiters = match ManualRateLimiters::new(
                rate_limit_config_arc,
            ) {
                Ok(limiters) => {
                    info!("ManualRateLimiters created.");
                    limiters
                }
                Err(e) => {
                    // If rate limiter creation fails due to config errors,
                    // it indicates a critical setup issue. Panic to prevent
                    // the server starting with invalid configuration.
                    error!(error = %e, "Failed to create ManualRateLimiters due to configuration error. Panicking.");
                    panic!("Invalid rate limiter configuration: {}", e);
                }
            };

            // - Create AppState
            #[cfg(all(feature = "chain", feature = "archive"))]
            // Assumes all adapters needed if features enabled
            let app_state: Option<
                Arc<crate::jsonrpc::infrastructure::state::AppState>,
            > = {
                use crate::jsonrpc::infrastructure::state::AppState;
                info!("Creating AppState with all adapters...");
                // Ensure all required adapter variables exist due to the cfg
                // attribute network_adapter and vm_adapter are
                // Arc<impl Adapter>, db/archive adapters are Arc<dyn Adapter>
                // AppState::new expects Arc<dyn Adapter>, so we might need to
                // explicitly cast Arc<impl> to Arc<dyn>
                // Note: Rust often does this implicitly, but let's be explicit
                // if needed.
                let app_state_instance = AppState::new(
                    config.clone(),  // Pass owned config
                    db_adapter,      // Already Arc<dyn DatabaseAdapter>
                    archive_adapter, // Already Arc<dyn ArchiveAdapter>
                    network_adapter, /* Assuming Arc<RuskNetworkAdapter>
                                      * implicitly converts to Arc<dyn
                                      * NetworkAdapter> */
                    vm_adapter, /* Assuming Arc<RuskVmAdapter> implicitly
                                 * converts to Arc<dyn VmAdapter> */
                    subscription_manager, // Pass owned manager
                    metrics_collector,    // Pass owned collector
                    manual_rate_limiters, // Pass owned limiters
                );
                let app_state_arc = Arc::new(app_state_instance);
                info!("AppState created successfully.");
                Some(app_state_arc)
            };

            // Placeholder for chain only (no archive)
            #[cfg(all(feature = "chain", not(feature = "archive")))]
            let app_state: Option<
                Arc<crate::jsonrpc::infrastructure::state::AppState>,
            > = {
                warn!("AppState creation without 'archive' feature is not fully implemented yet.");
                // TODO: Implement AppState::new or adjust AppState to handle
                // missing archive_adapter For now, return None
                // to prevent server start
                None
            };

            // Placeholder for neither chain nor archive (maybe basic health
            // check?)
            #[cfg(not(feature = "chain"))]
            // Implies not archive either if archive depends on chain
            let app_state: Option<
                Arc<crate::jsonrpc::infrastructure::state::AppState>,
            > = {
                warn!("AppState creation without 'chain' feature is not supported.");
                None
            };

            // - Spawn run_server task (only if app_state was created)
            if let Some(state) = app_state {
                info!("AppState created. Spawning JSON-RPC server task...");
                tokio::spawn(async move {
                    info!("JSON-RPC server task started.");
                    // Assuming run_server takes Arc<AppState>
                    if let Err(e) =
                        crate::jsonrpc::server::run_server(state).await
                    {
                        error!(error = %e, "JSON-RPC server failed");
                    } else {
                        info!("JSON-RPC server task finished gracefully.");
                    }
                });
            } else {
                error!("AppState could not be created due to missing features or errors. JSON-RPC server will not start.");
            }
        } else {
            info!("JSON-RPC server is disabled, skipping component initialization.");
        }
        // --- Conditionally Initialize JSON-RPC Components --- END ---

        Ok(())
    }
}
