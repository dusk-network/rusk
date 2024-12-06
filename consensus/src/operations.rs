// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::time::Duration;

use bytecheck::CheckBytes;
use execution_core::CommitRoot;
use node_data::bls::PublicKey;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{
    Block, Fault, Header, Slash, SpentTransaction, Transaction,
};
use node_data::StepName;
use rkyv::{Archive, Deserialize, Serialize};

use crate::errors::*;
use crate::merkle::Hash;

#[derive(
    Debug,
    Copy,
    Clone,
    Archive,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StateRoot(Hash);

impl StateRoot {
    pub fn from(h: Hash) -> Self {
        Self(h)
    }
    pub fn from_bytes(a: [u8; 32]) -> Self {
        Self(Hash::from(a))
    }
    // pub fn as_hash(&self) -> &Hash {
    //     &self.0
    // }
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
    pub fn as_commit_root(&self) -> CommitRoot {
        CommitRoot::from_bytes(*self.0.as_bytes())
    }
    pub fn from_commit_root(commit_root: &CommitRoot) -> Self {
        StateRoot::from_bytes(*commit_root.as_bytes())
    }
}

pub type EventBloom = [u8; 256];
pub type Voter = (PublicKey, usize);

#[derive(Default, Clone, Debug)]
pub struct CallParams {
    pub round: u64,
    pub generator_pubkey: node_data::bls::PublicKey,
    pub to_slash: Vec<Slash>,
    pub voters_pubkey: Vec<Voter>,
    pub max_txs_bytes: usize,
}

#[derive(Default)]
pub struct Output {
    pub txs: Vec<SpentTransaction>,
    pub verification_output: VerificationOutput,
    pub discarded_txs: Vec<Transaction>,
}

#[derive(Debug, PartialEq)]
pub struct VerificationOutput {
    pub state_root: StateRoot,
    pub event_bloom: EventBloom,
}

impl Default for VerificationOutput {
    fn default() -> Self {
        Self {
            state_root: StateRoot::from_bytes([0u8; 32]),
            event_bloom: [0u8; 256],
        }
    }
}

impl fmt::Display for VerificationOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerificationOutput {{ state_root: {}, event_bloom: {} }}",
            hex::encode(self.state_root.as_bytes()),
            hex::encode(self.event_bloom)
        )
    }
}

#[async_trait::async_trait]
pub trait Operations: Send + Sync {
    async fn verify_candidate_header(
        &self,
        candidate_header: &Header,
        expected_generator: &PublicKeyBytes,
    ) -> Result<(u8, Vec<Voter>, Vec<Voter>), HeaderError>;

    async fn verify_faults(
        &self,
        block_height: u64,
        faults: &[Fault],
    ) -> Result<(), OperationError>;

    async fn verify_state_transition(
        &self,
        blk: &Block,
        voters: &[Voter],
    ) -> Result<VerificationOutput, OperationError>;

    async fn execute_state_transition(
        &self,
        params: CallParams,
    ) -> Result<Output, OperationError>;

    async fn add_step_elapsed_time(
        &self,
        round: u64,
        step_name: StepName,
        elapsed: Duration,
    ) -> Result<(), OperationError>;

    async fn get_block_gas_limit(&self) -> u64;
}
