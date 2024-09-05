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

use events::ChainEventStreamer;
use execution_core::{dusk, Dusk};
use kadcast::config::Config as KadcastConfig;
use node::chain::ChainSrv;
use node::database::rocksdb::{self, Backend};
use node::database::{DatabaseOptions, DB};
use node::databroker::conf::Params as BrokerParam;
use node::databroker::DataBrokerSrv;
use node::mempool::conf::Params as MempoolParam;
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::telemetry::TelemetrySrv;
use node::{LongLivedService, Node};
use parking_lot::RwLock;
use rusk_abi::VM;
use tokio::sync::{broadcast, mpsc};

use crate::http::{HandleRequest, RuesEvent};

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
    pub(crate) charge_per_deploy_byte: Option<u64>,
    pub(crate) feeder_gas_limit: u64,
    pub(crate) block_gas_limit: u64,
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
    mempool: MempoolParam,
    telemetry_address: Option<String>,
    db_path: PathBuf,
    db_options: DatabaseOptions,
    max_chain_queue_size: usize,

    node: Option<node::Node<Kadcast<255>, Backend, Rusk>>,
    rusk: Rusk,
    rues_sender: Option<broadcast::Sender<RuesEvent>>,
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
        let (sender, node_receiver) = mpsc::channel(1000);
        let mut service_list: Vec<Box<Services>> = vec![
            Box::new(MempoolSrv::new(self.mempool, sender.clone())),
            Box::new(ChainSrv::new(
                self.consensus_keys_path,
                self.max_chain_queue_size,
                sender.clone(),
            )),
            Box::new(DataBrokerSrv::new(self.databroker)),
            Box::new(TelemetrySrv::new(self.telemetry_address)),
        ];
        if let Some(rues_sender) = self.rues_sender {
            service_list.push(Box::new(ChainEventStreamer {
                rues_sender,
                node_receiver,
            }))
        }
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

/// The emission schedule works as follows:
///   - the emission follows a Bitcoin-like halving function
///   - a total 499_782_528 Dusk will be emitted over 36 years divided in 9
///     periods of 4 years each
///
/// Considering the target block rate of 10 seconds, we assume a production of
/// 8640 blocks per day, which corresponds to 12_614_400 blocks per period.

// Target block production per day, assuming a block rate of 10 seconds
const BLOCKS_PER_DAY: u64 = 8640;
// Target block production per 4-year period
const BLOCKS_PER_PERIOD: u64 = BLOCKS_PER_DAY * 365 * 4;
// Block emission for each period, following the halving function
const BLOCK_EMISSIONS: [f64; 9] =
    [19.86, 9.93, 4.96, 2.48, 1.24, 0.62, 0.31, 0.15, 0.07];

// Returns the block emission for a certain height
pub const fn emission_amount(block_height: u64) -> Dusk {
    if block_height == 0 {
        return dusk(0.0);
    }

    let period = (block_height - 1) / BLOCKS_PER_PERIOD;
    match period {
        0..=8 => dusk(BLOCK_EMISSIONS[period as usize]),
        _ => dusk(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_amount() {
        // Test genesis block
        let genesis_emission = emission_amount(0);
        assert_eq!(
            genesis_emission,
            dusk(0.0),
            "For genesis block expected emission 0.0, but got {}",
            genesis_emission
        );

        // Block range and expected emission for each period
        let test_cases = vec![
            (1, 12_614_400, dusk(19.86)),           // Period 1
            (12_614_401, 25_228_800, dusk(9.93)),   // Period 2
            (25_228_801, 37_843_200, dusk(4.96)),   // Period 3
            (37_843_201, 50_457_600, dusk(2.48)),   // Period 4
            (50_457_601, 63_072_000, dusk(1.24)),   // Period 5
            (63_072_001, 75_686_400, dusk(0.62)),   // Period 6
            (75_686_401, 88_300_800, dusk(0.31)),   // Period 7
            (88_300_801, 100_915_200, dusk(0.15)),  // Period 8
            (100_915_201, 113_529_600, dusk(0.07)), // Period 9
            (113_529_601, u64::MAX, dusk(0.0)),     // Beyond period 9
        ];

        // Test emission periods
        for (start_block, end_block, expected_emission) in test_cases {
            // Test the first block in the range
            let emission_start = emission_amount(start_block);
            assert_eq!(
                emission_start, expected_emission,
                "For block height {} expected emission {}, but got {}",
                start_block, expected_emission, emission_start
            );

            // Test the last block in the range
            let emission_end = emission_amount(end_block);
            assert_eq!(
                emission_end, expected_emission,
                "For block height {} expected emission {}, but got {}",
                end_block, expected_emission, emission_end
            );
        }
    }

    const EXPECTED_PERIOD_EMISSIONS: [u64; 9] = [
        250_521_984, // Period 1
        125_260_992, // Period 2
        62_567_424,  // Period 3
        31_283_712,  // Period 4
        15_641_856,  // Period 5
        7_820_928,   // Period 6
        3_910_464,   // Period 7
        1_892_160,   // Period 8
        883_008,     // Period 9
    ];

    #[test]
    fn test_period_emissions() {
        // Check each period emission corresponds to the expected value
        for (i, &expected) in EXPECTED_PERIOD_EMISSIONS.iter().enumerate() {
            let block_emission = BLOCK_EMISSIONS[i];
            let period_emission =
                (block_emission * BLOCKS_PER_PERIOD as f64) as u64;
            assert_eq!(
                period_emission,
                expected,
                "Emission for period {} did not match: expected {}, got {}",
                i + 1,
                expected,
                period_emission
            );
        }
    }

    #[test]
    fn test_total_emission() {
        // Expected total emission based on the schedule
        let expected_total = 499_782_528u64;

        // Loop through each block emission and calculate the total emission
        let mut total_emission = 0u64;
        for &be in BLOCK_EMISSIONS.iter() {
            total_emission += (be * BLOCKS_PER_PERIOD as f64) as u64;
        }

        // Ensure the calculated total matches the expected total
        assert_eq!(
            total_emission, expected_total,
            "Total emission did not match the expected value"
        );
    }
}
