// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod rusk;
mod vm;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use execution_core::{dusk, Dusk};
use kadcast::config::Config as KadcastConfig;
use node::chain::ChainSrv;
use node::database::rocksdb::{self, Backend};
use node::database::DatabaseOptions;
use node::database::DB;
use node::databroker::conf::Params as BrokerParam;
use node::databroker::DataBrokerSrv;
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::telemetry::TelemetrySrv;
use node::{LongLivedService, Node};
use parking_lot::RwLock;
use rusk_abi::VM;
use tokio::sync::broadcast;

use crate::http::{HandleRequest, RuesEvent};

pub const MINIMUM_STAKE: Dusk = dusk(1000.0);

#[derive(Debug, Clone, Copy)]
pub struct RuskTip {
    pub current: [u8; 32],
    pub base: [u8; 32],
}

#[derive(Clone)]
pub struct Rusk {
    pub(crate) tip: Arc<RwLock<RuskTip>>,
    pub(crate) vm: Arc<VM>,
    dir: PathBuf,
    pub(crate) generation_timeout: Option<Duration>,
    pub(crate) gas_per_deploy_byte: Option<u64>,
    pub(crate) feeder_gas_limit: u64,
    pub(crate) event_sender: broadcast::Sender<RuesEvent>,
}

type Services = dyn LongLivedService<Kadcast<255>, rocksdb::Backend, Rusk>;

pub struct RuskNode {
    inner: node::Node<Kadcast<255>, Backend, Rusk>,
}

#[derive(Clone)]
pub struct RuskNodeBuilder {
    consensus_keys_path: String,
    databroker: BrokerParam,
    kadcast: KadcastConfig,
    telemetry_address: Option<String>,
    db_path: PathBuf,
    db_options: DatabaseOptions,
    node: Option<node::Node<Kadcast<255>, Backend, Rusk>>,
    rusk: Rusk,
}

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

    pub fn new(rusk: Rusk) -> Self {
        Self {
            rusk,
            node: Default::default(),
            db_path: Default::default(),
            db_options: Default::default(),
            kadcast: Default::default(),
            consensus_keys_path: Default::default(),
            databroker: Default::default(),
            telemetry_address: Default::default(),
        }
    }

    pub fn build_data_sources(
        &mut self,
    ) -> anyhow::Result<Vec<Box<dyn HandleRequest>>> {
        let sources: Vec<Box<dyn HandleRequest>> = vec![
            Box::new(self.rusk.clone()),
            Box::new(self.get_or_create_node()?),
        ];
        Ok(sources)
    }

    fn get_or_create_node(&mut self) -> anyhow::Result<RuskNode> {
        if self.node.is_none() {
            let db = rocksdb::Backend::create_or_open(
                self.db_path.clone(),
                self.db_options.clone(),
            );
            let net = Kadcast::new(self.kadcast.clone())?;
            let node = Node::new(net, db, self.rusk.clone());
            self.node = Some(node)
        }
        Ok(RuskNode {
            inner: self.node.clone().expect("Node to be initialized"),
        })
    }

    pub async fn build_and_run(mut self) -> anyhow::Result<()> {
        let node = self.get_or_create_node()?;
        let mut service_list: Vec<Box<Services>> = vec![
            Box::<MempoolSrv>::default(),
            Box::new(ChainSrv::new(self.consensus_keys_path)),
            Box::new(DataBrokerSrv::new(self.databroker)),
            Box::new(TelemetrySrv::new(self.telemetry_address)),
        ];
        node.inner.initialize(&mut service_list).await?;
        node.inner.spawn_all(service_list).await?;
        Ok(())
    }
}

impl RuskNode {
    pub fn db(&self) -> Arc<tokio::sync::RwLock<Backend>> {
        self.inner.database() as Arc<tokio::sync::RwLock<Backend>>
    }

    pub fn network(&self) -> Arc<tokio::sync::RwLock<Kadcast<255>>> {
        self.inner.network() as Arc<tokio::sync::RwLock<Kadcast<255>>>
    }
}

/// Calculates the value that the coinbase notes should contain.
///
/// 10% of the reward value goes to the Dusk address (rounded down).
/// 70% of the reward value is considered fixed reward for Block Generator.
/// 10% of the reward value is considered extra reward for Block Generator.
/// 10% of the reward value goes to the all validators/voters of previous block
/// (rounded down).
const fn coinbase_value(
    block_height: u64,
    dusk_spent: u64,
) -> (Dusk, Dusk, Dusk, Dusk) {
    let reward_value = emission_amount(block_height) + dusk_spent;
    let one_tenth_reward = reward_value / 10;

    let dusk_value = one_tenth_reward;
    let voters_value = one_tenth_reward;
    let generator_extra_value = one_tenth_reward;

    let generator_fixed_value =
        reward_value - dusk_value - voters_value - generator_extra_value;

    (
        dusk_value,
        generator_fixed_value,
        generator_extra_value,
        voters_value,
    )
}

/// This implements the emission schedule described in the economic paper.
pub const fn emission_amount(block_height: u64) -> Dusk {
    match block_height {
        1..=12_500_000 => dusk(16.0),
        12_500_001..=18_750_000 => dusk(12.8),
        18_750_001..=25_000_000 => dusk(9.6),
        25_000_001..=31_250_000 => dusk(8.0),
        31_250_001..=37_500_000 => dusk(6.4),
        37_500_001..=43_750_000 => dusk(4.8),
        43_750_001..=50_000_000 => dusk(3.2),
        50_000_001..=62_500_000 => dusk(1.6),
        _ => dusk(0.0),
    }
}
