// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::time::Duration;

use node_data::bls::{PublicKey, PublicKeyBytes};
use node_data::ledger::{Block, Fault, Header, Slash, SpentTransaction};
use node_data::StepName;

use crate::errors::*;

pub type StateRoot = [u8; 32];
pub type EventBloom = [u8; 256];
pub type Voter = (PublicKey, usize);

#[derive(Default, Clone, Debug)]
pub struct StateTransitionData {
    pub round: u64,
    pub generator: node_data::bls::PublicKey,
    pub slashes: Vec<Slash>,
    pub cert_voters: Vec<Voter>,
    pub max_txs_bytes: usize,
    pub prev_state_root: StateRoot,
}

#[derive(Debug, PartialEq)]
pub struct StateTransitionResult {
    pub state_root: StateRoot,
    pub event_bloom: EventBloom,
}

impl Default for StateTransitionResult {
    fn default() -> Self {
        Self {
            state_root: [0u8; 32],
            event_bloom: [0u8; 256],
        }
    }
}

impl fmt::Display for StateTransitionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State transition result: {{ state_root: {}, event_bloom: {} }}",
            hex::encode(self.state_root),
            hex::encode(self.event_bloom)
        )
    }
}

#[async_trait::async_trait]
pub trait Operations: Send + Sync {
    async fn validate_block_header(
        &self,
        candidate_header: &Header,
        expected_generator: &PublicKeyBytes,
    ) -> Result<Vec<Voter>, HeaderError>;

    async fn validate_faults(
        &self,
        block_height: u64,
        faults: &[Fault],
    ) -> Result<(), OperationError>;

    async fn validate_state_transition(
        &self,
        prev_state: StateRoot,
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<(), OperationError>;

    async fn generate_state_transition(
        &self,
        transition_data: StateTransitionData,
    ) -> Result<(Vec<SpentTransaction>, StateTransitionResult), OperationError>;

    async fn add_step_elapsed_time(
        &self,
        round: u64,
        step_name: StepName,
        elapsed: Duration,
    ) -> Result<(), OperationError>;

    async fn get_block_gas_limit(&self) -> u64;
}
