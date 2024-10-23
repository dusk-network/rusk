// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::operations::Voter;
use dusk_consensus::{
    operations::{CallParams, VerificationOutput},
    user::{provisioners::Provisioners, stake::Stake},
};
use execution_core::signatures::bls::PublicKey as BlsPublicKey;
use execution_core::transfer::data::ContractBytecode;
use execution_core::transfer::moonlight::AccountData;
use node_data::events::contract::ContractEvent;
use node_data::ledger::{Block, SpentTransaction, Transaction};

#[derive(Default)]
pub struct Config {}

pub trait VMExecution: Send + Sync + 'static {
    fn execute_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        params: &CallParams,
        txs: I,
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        Vec<Transaction>,
        VerificationOutput,
    )>;

    fn verify_state_transition(
        &self,
        blk: &Block,
        voters: &[Voter],
    ) -> anyhow::Result<VerificationOutput>;

    fn accept(
        &self,
        blk: &Block,
        voters: &[Voter],
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        VerificationOutput,
        Vec<ContractEvent>,
    )>;

    fn finalize_state(
        &self,
        commit: [u8; 32],
        to_delete: Vec<[u8; 32]>,
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

// Returns gas charge for bytecode deployment.
pub fn bytecode_charge(
    bytecode: &ContractBytecode,
    gas_per_deploy_byte: u64,
) -> u64 {
    bytecode.bytes.len() as u64 * gas_per_deploy_byte
}
