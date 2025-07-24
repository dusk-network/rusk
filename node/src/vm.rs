// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::errors::StateTransitionError;
use dusk_consensus::operations::{
    StateTransitionData, StateTransitionResult, Voter,
};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::stake::Stake;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::{Block, SpentTransaction, Transaction};

#[derive(Default)]
pub struct Config {}

pub trait VMExecution: Send + Sync + 'static {
    fn create_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        transition_data: &StateTransitionData,
        mempool_txs: I,
    ) -> Result<
        (
            Vec<SpentTransaction>,
            Vec<Transaction>,
            StateTransitionResult,
        ),
        StateTransitionError,
    >;

    fn verify_state_transition(
        &self,
        prev_state: [u8; 32],
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<(), StateTransitionError>;

    fn accept_state_transition(
        &self,
        prev_state: [u8; 32],
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<
        (Vec<SpentTransaction>, Vec<ContractTxEvent>),
        StateTransitionError,
    >;

    fn finalize_state(
        &self,
        commit: [u8; 32],
        to_merge: Vec<[u8; 32]>,
    ) -> anyhow::Result<()>;

    fn preverify(
        &self,
        tx: &Transaction,
    ) -> anyhow::Result<PreverificationResult>;

    fn get_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Provisioners>;

    fn get_changed_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Vec<(node_data::bls::PublicKey, Option<Stake>)>>;

    fn get_provisioner(
        &self,
        pk: &BlsPublicKey,
    ) -> anyhow::Result<Option<Stake>>;

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]>;

    fn move_to_commit(&self, commit: [u8; 32]) -> anyhow::Result<()>;

    /// Returns last finalized state root
    fn get_finalized_state_root(&self) -> anyhow::Result<[u8; 32]>;

    /// Returns block gas limit
    fn get_block_gas_limit(&self) -> u64;

    fn revert(&self, state_hash: [u8; 32]) -> anyhow::Result<[u8; 32]>;
    fn revert_to_finalized(&self) -> anyhow::Result<[u8; 32]>;

    fn gas_per_deploy_byte(&self) -> u64;
    fn min_deployment_gas_price(&self) -> u64;
    fn min_gas_limit(&self) -> u64;
    fn min_deploy_points(&self) -> u64;

    fn gas_per_blob(&self) -> u64;
    fn blob_activation_height(&self) -> u64;
}

#[allow(clippy::large_enum_variant)]
pub enum PreverificationResult {
    Valid,
    // Current account state, nonce used by tx
    FutureNonce {
        account: BlsPublicKey,
        state: AccountData,
        nonce_used: u64,
    },
}
