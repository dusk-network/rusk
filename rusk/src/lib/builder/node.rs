// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use kadcast::config::Config as KadcastConfig;
#[cfg(feature = "archive")]
use node::archive::conf::Params as ArchiveParam;
use node::chain::ChainSrv;
use node::database::rocksdb::MD_HASH_KEY;
use node::database::{DB, DatabaseOptions, Ledger, Metadata, rocksdb};
use node::databroker::DataBrokerSrv;
use node::databroker::conf::Params as BrokerParam;
use node::mempool::MempoolSrv;
use node::mempool::conf::Params as MempoolParam;
use node::network::Kadcast;
use node::telemetry::TelemetrySrv;
use node::{LongLivedService, Node};
use node_data::ledger::{Header, to_str};
use tokio::sync::{broadcast, mpsc};
use tracing::info;
#[cfg(feature = "archive")]
use {dusk_bytes::Serializable, node::archive::Archive, tracing::debug};

use crate::http::{HttpHandlers, HttpServer, HttpServerConfig};
use crate::node::{
    ChainEventStreamer, DriverStore, RuskNode, RuskOptVmConfig, RuskVmConfig,
    Services, WellKnownVmConfig,
};
use crate::{Rusk, VERSION};

/// Finds the stored block header matching `state_root`.
///
/// Returns `Ok(None)` only when the chain DB has no tip metadata stored yet,
/// which means the consumer must decide how to initialize the header. If tip
/// metadata exists, the matching header must be present and missing data is
/// reported as an error.
fn find_block_header_by_state_root<DB>(
    db: &DB,
    state_root: [u8; 32],
) -> crate::Result<Option<Header>>
where
    DB: node::database::DB,
{
    db.view(|db| {
        if db
            .op_read(MD_HASH_KEY)
            .map_err(|err| io::Error::other(format!("{err}")))?
            .is_none()
        {
            return Ok(None);
        }

        let latest = db
            .latest_block()
            .map_err(|err| io::Error::other(format!("{err}")))?;

        let mut height = latest.header.height;
        loop {
            let block = db
                .block_by_height(height)
                .map_err(|err| io::Error::other(format!("{err}")))?
                .ok_or_else(|| {
                    io::Error::other(format!(
                        "Cannot load block at height {height}"
                    ))
                })?;
            let header = block.header();

            if header.state_hash == state_root {
                return Ok(Some(header.clone()));
            }

            if height == 0 {
                return Err(io::Error::other(format!(
                    "Cannot find block header for state root {}",
                    to_str(&state_root)
                ))
                .into());
            }

            height -= 1;
        }
    })
}

#[derive(Default)]
pub struct RuskNodeBuilder {
    #[cfg(feature = "archive")]
    archive: ArchiveParam,
    consensus_keys_path: String,
    databroker: BrokerParam,
    kadcast: KadcastConfig,
    mempool: MempoolParam,
    telemetry_address: Option<String>,
    db_path: PathBuf,
    db_options: DatabaseOptions,
    max_chain_queue_size: usize,
    genesis_timestamp: u64,
    vm_config: RuskOptVmConfig,
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
    #[cfg(feature = "archive")]
    pub fn with_archive(mut self, conf: ArchiveParam) -> Self {
        self.archive = conf;
        self
    }

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

    pub fn with_min_gas_limit(mut self, min_gas_limit: Option<u64>) -> Self {
        self.min_gas_limit = min_gas_limit;
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

    pub fn with_vm_config(mut self, vm_config: RuskOptVmConfig) -> Self {
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
    pub async fn build_and_run(mut self) -> anyhow::Result<()> {
        let channel_cap = self
            .http
            .as_ref()
            .map(|h| h.ws_event_channel_cap)
            .unwrap_or(1);
        // HTTP and chain event streaming create fresh receivers from this
        // sender.
        let (rues_sender, _) = broadcast::channel(channel_cap);
        let (node_sender, node_receiver) = mpsc::channel(1000);

        let chain_id = self.kadcast.kadcast_id.unwrap_or_default();
        let known_conf = WellKnownVmConfig::from_chain_id(chain_id);
        self.vm_config.inject_network_conf(known_conf);

        let vm_config = RuskVmConfig::try_from(self.vm_config)?;
        #[cfg(feature = "archive")]
        let archive = Archive::create_or_open_with_conf(
            self.db_path.clone(),
            self.archive,
        )
        .await;

        let min_gas_limit = self.min_gas_limit.unwrap_or(DEFAULT_MIN_GAS_LIMIT);
        let finality_activation = vm_config
            .feature(crate::node::FEATURE_ABI_PUBLIC_SENDER)
            .map(|f| f.unwrap_height())
            .unwrap_or(u64::MAX);

        let mut module_shading = HashMap::new();
        for (feat, activation) in vm_config.features() {
            let feat = feat.to_ascii_lowercase();
            if let Some(contract_id) = feat.strip_prefix("shade_") {
                let contract_id = contract_id.to_string().try_into()?;
                module_shading
                    .insert(contract_id, activation.unwrap_ranges().to_vec());
            }
        }

        let blob_expire_after =
            self.blob_expire_after.unwrap_or(DEFAULT_BLOB_EXPIRE_AFTER);

        let db = rocksdb::Backend::create_or_open(
            self.db_path.clone(),
            self.db_options.clone(),
        );

        let rusk = Rusk::new(
            self.state_dir,
            |state_root| {
                let header = find_block_header_by_state_root(&db, state_root)?
                    .unwrap_or_else(|| {
                        node::chain::genesis_block(
                            state_root,
                            self.genesis_timestamp,
                        )
                        .header()
                        .clone()
                    });

                Ok(header)
            },
            self.kadcast.kadcast_id.unwrap_or_default(),
            vm_config,
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
            let net = Kadcast::new(self.kadcast)?;
            let future_nonce_retry_queue =
                node::mempool::FutureNonceRetryHandle::new(
                    self.mempool.max_queue_size,
                    self.mempool.max_moonlight_future_nonce_per_account,
                );
            RuskNode::new(
                Node::new(net, db, rusk.clone()),
                future_nonce_retry_queue.clone(),
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
            module_shading,
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
            Box::new(MempoolSrv::with_future_nonce_retry_queue(
                self.mempool,
                node_sender.clone(),
                node.future_nonce_retry_queue(),
            )),
            Box::new(chain_srv),
            Box::new(DataBrokerSrv::new(self.databroker)),
            Box::new(TelemetrySrv::new(self.telemetry_address)),
        ];

        let mut _ws_server = None;
        if let Some(http) = self.http {
            info!("Configuring HTTP");

            service_list.push(Box::new(ChainEventStreamer {
                node_receiver,
                rues_sender: rues_sender.clone(),
            }));

            let mut services = HttpHandlers::default();
            services.set_rusk_handler(rusk.clone());
            services.set_chain_handler(node.clone());
            services.set_graphql_handler(node.clone());

            #[cfg(feature = "prover")]
            services.set_prover_handler(rusk_prover::LocalProver);

            _ws_server =
                Some(HttpServer::bind(services, rues_sender, http).await?);
        }

        node.inner().initialize(&mut service_list).await?;

        #[cfg(feature = "archive")]
        {
            if archive.fetch_active_accounts().await? == 0 {
                let base_header = None;
                let accounts = rusk.moonlight_accounts(base_header);

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

#[cfg(test)]
mod tests {
    use node::database::{DB as _, Ledger as _};
    use node_data::ledger::Label;
    use tempfile::tempdir;

    use super::*;

    fn test_header(height: u64, state: u8, hash: u8, prev_hash: u8) -> Header {
        Header {
            height,
            state_hash: [state; 32],
            hash: [hash; 32],
            prev_block_hash: [prev_hash; 32],
            ..Default::default()
        }
    }

    fn store_header(
        db: &rocksdb::Backend,
        header: &Header,
    ) -> crate::Result<()> {
        db.update(|tx| {
            tx.store_block(header, &[], &[], Label::Final(0))?;
            Ok(())
        })
        .map_err(|err| io::Error::other(format!("{err}")))?;
        Ok(())
    }

    // Covers the restart path where the persisted finalized state root is
    // behind the chain DB tip and must be matched by walking back headers.
    #[test]
    fn finds_header_by_state_root_before_tip() -> crate::Result<()> {
        let dir = tempdir()?;
        let db = rocksdb::Backend::create_or_open(
            dir.path(),
            DatabaseOptions::default(),
        );

        let genesis = test_header(0, 1, 10, 0);
        let block_one = test_header(1, 2, 11, 10);
        let tip = test_header(2, 3, 12, 11);

        store_header(&db, &genesis)?;
        store_header(&db, &block_one)?;
        store_header(&db, &tip)?;

        let recovered =
            find_block_header_by_state_root(&db, block_one.state_hash)?
                .expect("header should be found");

        assert_eq!(recovered.height, block_one.height);
        assert_eq!(recovered.hash, block_one.hash);
        assert_eq!(recovered.state_hash, block_one.state_hash);

        Ok(())
    }

    // Covers corrupted/incomplete chain metadata: once tip metadata exists,
    // a missing state root must be reported instead of falling back to genesis.
    #[test]
    fn errors_when_metadata_exists_but_state_root_is_missing()
    -> crate::Result<()> {
        let dir = tempdir()?;
        let db = rocksdb::Backend::create_or_open(
            dir.path(),
            DatabaseOptions::default(),
        );

        let genesis = test_header(0, 1, 10, 0);
        let tip = test_header(1, 2, 11, 10);

        store_header(&db, &genesis)?;
        store_header(&db, &tip)?;

        let err = find_block_header_by_state_root(&db, [99; 32])
            .expect_err("missing state root should be an error");

        assert!(
            err.to_string().contains("Cannot find block header"),
            "unexpected error: {err}"
        );

        Ok(())
    }

    // Covers first-start initialization where no chain metadata exists yet and
    // the builder decides to use the genesis header fallback.
    #[test]
    fn empty_db_allows_genesis_header_fallback() -> crate::Result<()> {
        let dir = tempdir()?;
        let db = rocksdb::Backend::create_or_open(
            dir.path(),
            DatabaseOptions::default(),
        );

        let state_root = [42; 32];
        let timestamp = 1234;
        let recovered = find_block_header_by_state_root(&db, state_root)?
            .unwrap_or_else(|| {
                node::chain::genesis_block(state_root, timestamp)
                    .header()
                    .clone()
            });

        assert_eq!(recovered.height, 0);
        assert_eq!(recovered.timestamp, timestamp);
        assert_eq!(recovered.state_hash, state_root);

        Ok(())
    }
}
