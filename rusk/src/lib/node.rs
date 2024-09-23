// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod events;
mod rusk;
mod vm;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use execution_core::{dusk, Dusk};
use kadcast::config::Config as KadcastConfig;

use node::chain::ChainSrv;

use node::database::rocksdb::{self, Backend};
use node::network::Kadcast;
use node::LongLivedService;
use parking_lot::RwLock;
use rusk_abi::VM;
use tokio::sync::{broadcast, mpsc};
#[cfg(feature = "archive")]
use {
    node::archivist::ArchivistSrv, node::database::archive::SQLiteArchive,
    node_data::archive::ArchivalData,
};

use crate::http::RuesEvent;

pub(crate) use events::ChainEventStreamer;
pub use vm::ContractTxEvent;

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
    pub(crate) chain_id: u8,
    pub(crate) generation_timeout: Option<Duration>,
    pub(crate) gas_per_deploy_byte: Option<u64>,
    pub(crate) min_deployment_gas_price: Option<u64>,
    pub(crate) feeder_gas_limit: u64,
    pub(crate) block_gas_limit: u64,
    pub(crate) event_sender: broadcast::Sender<RuesEvent>,
    #[cfg(feature = "archive")]
    pub(crate) archive_sender: mpsc::Sender<ArchivalData>,
}

pub(crate) type Services =
    dyn LongLivedService<Kadcast<255>, rocksdb::Backend, Rusk>;

#[derive(Clone)]
pub struct RuskNode {
    inner: node::Node<Kadcast<255>, Backend, Rusk>,
}

impl RuskNode {
    /// Run the RuskNode
    pub async fn run(
        &mut self,
        mut service_list: Vec<Box<Services>>,
    ) -> anyhow::Result<()> {
        self.inner.initialize(&mut service_list).await?;
        self.inner.spawn_all(service_list).await?;

        Ok(())
    }

    pub fn new(inner: node::Node<Kadcast<255>, Backend, Rusk>) -> Self {
        Self { inner }
}

pub struct RuskNodeBuilder {
    consensus_keys_path: String,
    databroker: BrokerParam,
    kadcast: KadcastConfig,
    mempool: MempoolParam,
    telemetry_address: Option<String>,
    db_path: PathBuf,
    db_options: DatabaseOptions,
    max_chain_queue_size: usize,

    node: Option<node::Node<Kadcast<255>, Backend, Rusk>>,
    rusk: Rusk,
    rues_sender: Option<broadcast::Sender<RuesEvent>>,
    #[cfg(feature = "archive")]
    archive_channel: (
        Option<tokio::sync::mpsc::Sender<ArchivalData>>,
        Option<tokio::sync::mpsc::Receiver<ArchivalData>>,
    ),
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

    pub fn with_rues(
        mut self,
        rues_sender: broadcast::Sender<RuesEvent>,
    ) -> Self {
        self.rues_sender = Some(rues_sender);
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

    #[cfg(feature = "archive")]
    pub fn with_archivist(
        mut self,
        archive_sender: tokio::sync::mpsc::Sender<ArchivalData>,
        archive_receiver: tokio::sync::mpsc::Receiver<ArchivalData>,
    ) -> Self {
        self.archive_channel = (Some(archive_sender), Some(archive_receiver));
        self
    }

    pub fn with_chain_queue_size(mut self, max_queue_size: usize) -> Self {
        self.max_chain_queue_size = max_queue_size;
        self
    }

    pub fn new(rusk: Rusk) -> Self {
        Self {
            consensus_keys_path: Default::default(),
            databroker: Default::default(),
            kadcast: Default::default(),
            mempool: Default::default(),
            telemetry_address: None,
            db_path: Default::default(),
            db_options: Default::default(),
            max_chain_queue_size: 0,
            node: None,
            rusk,
            rues_sender: None,
            #[cfg(feature = "archive")]
            archive_channel: (None, None),
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

    /// Build the RuskNode and corresponding services
    pub async fn build(
        mut self,
    ) -> anyhow::Result<(RuskNode, Vec<Box<Services>>)> {
        let node = self.get_or_create_node()?;

        let (node_sender, node_receiver) = mpsc::channel(1000);
        let mut service_list: Vec<Box<Services>> = vec![
            Box::new(MempoolSrv::new(self.mempool, node_sender.clone())),
            Box::new(ChainSrv::new(
                self.consensus_keys_path,
                self.max_chain_queue_size,
                node_sender.clone(),
            )),
            Box::new(DataBrokerSrv::new(self.databroker)),
            Box::new(TelemetrySrv::new(self.telemetry_address)),
        ];

        if let Some(rues_sender) = self.rues_sender {
            service_list.push(Box::new(ChainEventStreamer {
                rues_sender,
                node_receiver,
                #[cfg(feature = "archive")]
                archivist_sender: self
                    .archive_channel
                    .0
                    .expect("archive feature set but archive sender not set"),
            }))
        }

        #[cfg(feature = "archive")]
        if let (_, Some(archive_receiver)) = self.archive_channel {
            let archivist =
                SQLiteArchive::create_or_open(self.db_path.clone()).await;

            service_list.push(Box::new(ArchivistSrv {
                archive_receiver,
                archivist,
            }));
        } else {
            panic!("archive feature set but archivist not set")
        }

        Ok((node, service_list))
    }
}

impl RuskNode {
    pub fn db(&self) -> Arc<tokio::sync::RwLock<Backend>> {
        self.inner.database() as Arc<tokio::sync::RwLock<Backend>>
    }

    pub fn network(&self) -> Arc<tokio::sync::RwLock<Kadcast<255>>> {
        self.inner.network() as Arc<tokio::sync::RwLock<Kadcast<255>>>
    }

    pub fn inner(&self) -> &node::Node<Kadcast<255>, Backend, Rusk> {
        &self.inner
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

/// The emission schedule works as follows:
///   - the emission follows a Bitcoin-like halving function
///   - a total 500.000.000 Dusk will be emitted over 36 years divided in 9
///     periods of 4 years each
///
/// Considering the target block rate of 10 seconds, we assume a production of
/// 8640 blocks per day, which corresponds to 12_614_400 blocks per period.

// Returns the block emission for a certain height, following the halving
// function
pub const fn emission_amount(block_height: u64) -> Dusk {
    match block_height {
        0 => 0,                                     // Genesis
        1..=12_614_400 => dusk(19.8574),            // Period 1
        12_614_401..=25_228_800 => dusk(9.9287),    // Period 2
        25_228_801..=37_843_200 => dusk(4.96435),   // Period 3
        37_843_201..=50_457_600 => dusk(2.48218),   // Period 4
        50_457_601..=63_072_000 => dusk(1.24109),   // Period 5
        63_072_001..=75_686_400 => dusk(0.62054),   // Period 6
        75_686_401..=88_300_800 => dusk(0.31027),   // Period 7
        88_300_801..=100_915_200 => dusk(0.15514),  // Period 8
        100_915_201..=113_529_596 => dusk(0.07757), // Period 9
        113_529_597 => dusk(0.05428),               // Last mint
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Target block production per day, assuming a block rate of 10 seconds
    const BLOCKS_PER_DAY: u64 = 8640;
    // Target block production per 4-year period
    const BLOCKS_PER_PERIOD: u64 = BLOCKS_PER_DAY * 365 * 4;

    const EXPECTED_PERIOD_EMISSIONS: [u64; 10] = [
        dusk(250_489_186.56), // Period 1
        dusk(125_244_593.28), // Period 2
        dusk(62_622_296.64),  // Period 3
        dusk(31_311_211.392), // Period 4
        dusk(15_655_605.696), // Period 5
        dusk(7_827_739.776),  // Period 6
        dusk(3_913_869.888),  // Period 7
        dusk(1_956_998.016),  // Period 8
        dusk(978_498.752),    // Period 9
        dusk(0.0),            // After Period 9
    ];

    #[test]
    fn test_period_emissions() {
        // Check each period emission corresponds to the expected value
        for (period, &expected) in EXPECTED_PERIOD_EMISSIONS.iter().enumerate()
        {
            let start_block = (period as u64 * BLOCKS_PER_PERIOD) + 1;
            let end_block = start_block + BLOCKS_PER_PERIOD;
            let mut period_emission = 0;
            for height in start_block..end_block {
                period_emission += emission_amount(height);
            }
            assert_eq!(
                period_emission,
                expected,
                "Emission for period {} did not match: expected {}, got {}",
                period + 1,
                expected,
                period_emission
            );
        }
    }

    #[test]
    fn test_total_emission() {
        let mut total_emission = 0u64;
        // Loop through each block emission and calculate the total emission
        for h in 0..=BLOCKS_PER_PERIOD * 10 {
            total_emission += emission_amount(h)
        }
        // Expected total emission based on the schedule
        let expected_total = dusk(500_000_000.0);

        // Ensure the calculated total matches the expected total
        assert_eq!(
            total_emission, expected_total,
            "Total emission did not match the expected value"
        );
    }
}
