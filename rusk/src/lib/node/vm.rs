// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
mod query;

use dusk_consensus::errors::StateTransitionError;
use node_data::events::contract::ContractTxEvent;
use tracing::{debug, info};

use dusk_consensus::operations::{
    StateTransitionData, StateTransitionResult, Voter,
};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::stake::Stake;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::StakeData;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use node::vm::{PreverificationResult, VMExecution};
use node_data::bls::PublicKey;
use node_data::ledger::{Block, Header, SpentTransaction, Transaction};

use super::{RuesEvent, Rusk};
pub use config::feature::*;
pub use config::Config as RuskVmConfig;

use crate::Error as RuskError;

impl VMExecution for Rusk {
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
    > {
        self.create_state_transition(transition_data, mempool_txs)
    }

    /// Executes a block's state transition and checks its result against the
    /// block's header
    fn verify_state_transition(
        &self,
        prev_state: [u8; 32],
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<(), StateTransitionError> {
        debug!("Verifying state transition");

        // Execute state transition
        let (_, transition_result, _, _) =
            self.execute_state_transition(prev_state, blk, cert_voters)?;

        // Check result against header
        check_transition_result(&transition_result, blk.header())?;

        Ok(())
    }

    /// Execute and persist a block's state transition.
    ///
    /// # Arguments
    ///
    /// * `prev_state` - the root of the previous block's state.
    /// * `blk` - the block defining the state transition.
    /// * `cert_voters` - list of voters in the Certificate for the previous
    ///   block. This is used to compute rewards. It is passed as a separate
    ///   argument for convenience (voters are extracted during the Certificate
    ///   verification).
    ///
    /// # Returns
    ///
    /// * Vec<SpentTransaction> - The transactions that were spent.
    /// * Vec<ContractTxEvent> - All emitted contract events
    ///
    /// # Errors
    ///
    /// * If the state transition fails verification
    /// * If the session fails to commit
    fn accept_state_transition(
        &self,
        prev_state: [u8; 32],
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<
        (Vec<SpentTransaction>, Vec<ContractTxEvent>),
        StateTransitionError,
    > {
        debug!("Accepting state transition");

        // Execute state transition
        let (executed_txs, transition_result, contract_events, session) =
            self.execute_state_transition(prev_state, blk, cert_voters)?;

        // Check result against header
        check_transition_result(&transition_result, blk.header())?;

        // Commit state transition
        self.commit_session(session).map_err(|err| {
            StateTransitionError::PersistenceError(format!("{err}"))
        })?;

        // Send contract events to RUES
        // NOTE: we do it here and not in accept_block because RuesEvent is part
        // of the Rusk component
        for event in contract_events.clone() {
            let rues_event = RuesEvent::from(event);
            let _ = self.event_sender.send(rues_event);
        }

        Ok((executed_txs, contract_events))
    }

    fn move_to_commit(&self, commit: [u8; 32]) -> anyhow::Result<()> {
        self.query_session(Some(commit))
            .map_err(|e| anyhow::anyhow!("Cannot open session {e}"))?;
        self.set_current_commit(commit);
        Ok(())
    }

    fn finalize_state(
        &self,
        commit: [u8; 32],
        to_merge: Vec<[u8; 32]>,
    ) -> anyhow::Result<()> {
        debug!("Received finalize request");
        self.finalize_state(commit, to_merge)
            .map_err(|e| anyhow::anyhow!("Cannot finalize state: {e}"))
    }

    fn preverify(
        &self,
        tx: &Transaction,
    ) -> anyhow::Result<PreverificationResult> {
        info!("Received preverify request");
        let tx = &tx.inner;

        match tx {
            ProtocolTransaction::Phoenix(tx) => {
                let tx_nullifiers = tx.nullifiers().to_vec();
                let existing_nullifiers =
                    self.existing_nullifiers(&tx_nullifiers).map_err(|e| {
                        anyhow::anyhow!("Cannot check nullifiers: {e}")
                    })?;

                if !existing_nullifiers.is_empty() {
                    let err =
                        RuskError::RepeatingNullifiers(existing_nullifiers);
                    return Err(anyhow::anyhow!("{err}"));
                }

                if !has_unique_elements(tx_nullifiers) {
                    let err = RuskError::DoubleNullifiers;
                    return Err(anyhow::anyhow!("{err}"));
                }

                match crate::verifier::verify_proof(tx) {
                    Ok(true) => Ok(PreverificationResult::Valid),
                    Ok(false) => Err(anyhow::anyhow!("Invalid proof")),
                    Err(e) => {
                        Err(anyhow::anyhow!("Cannot verify the proof: {e}"))
                    }
                }
            }
            ProtocolTransaction::Moonlight(tx) => {
                let account_data = self.account(tx.sender()).map_err(|e| {
                    anyhow::anyhow!("Cannot check account: {e}")
                })?;

                let max_value = tx
                    .gas_limit()
                    .checked_mul(tx.gas_price())
                    .and_then(|v| v.checked_add(tx.value()))
                    .and_then(|v| v.checked_add(tx.deposit()))
                    .ok_or(anyhow::anyhow!("Value spent will overflow"))?;

                if max_value > account_data.balance {
                    return Err(anyhow::anyhow!(
                        "Value spent larger than account holds"
                    ));
                }

                if tx.nonce() <= account_data.nonce {
                    let err = RuskError::RepeatingNonce(
                        (*tx.sender()).into(),
                        tx.nonce(),
                    );
                    return Err(anyhow::anyhow!("{err}"));
                }

                let result = if tx.nonce() > account_data.nonce + 1 {
                    PreverificationResult::FutureNonce {
                        account: *tx.sender(),
                        state: account_data,
                        nonce_used: tx.nonce(),
                    }
                } else {
                    PreverificationResult::Valid
                };

                match crate::verifier::verify_signature(
                    tx.blob_to_memo().as_ref().unwrap_or(tx),
                ) {
                    Ok(true) => Ok(result),
                    Ok(false) => Err(anyhow::anyhow!("Invalid signature")),
                    Err(e) => {
                        Err(anyhow::anyhow!("Cannot verify the signature: {e}"))
                    }
                }
            }
        }
    }

    fn get_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Provisioners> {
        self.query_provisioners(Some(base_commit))
    }

    fn get_changed_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Vec<(PublicKey, Option<Stake>)>> {
        self.query_provisioners_change(Some(base_commit))
    }

    fn get_provisioner(
        &self,
        pk: &BlsPublicKey,
    ) -> anyhow::Result<Option<Stake>> {
        let stake = self
            .provisioner(pk)
            .map_err(|e| anyhow::anyhow!("Cannot get provisioner {e}"))?
            .map(Self::to_stake);
        Ok(stake)
    }

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.state_root())
    }

    fn get_finalized_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.base_root())
    }

    fn revert(&self, state_hash: [u8; 32]) -> anyhow::Result<[u8; 32]> {
        let state_hash = self
            .revert(state_hash)
            .map_err(|inner| anyhow::anyhow!("Cannot revert: {inner}"))?;

        Ok(state_hash)
    }

    fn revert_to_finalized(&self) -> anyhow::Result<[u8; 32]> {
        let state_hash = self.revert_to_base_root().map_err(|inner| {
            anyhow::anyhow!("Cannot revert to finalized: {inner}")
        })?;

        Ok(state_hash)
    }

    fn get_block_gas_limit(&self) -> u64 {
        self.vm_config.block_gas_limit
    }

    fn gas_per_deploy_byte(&self) -> u64 {
        self.vm_config.gas_per_deploy_byte
    }

    fn min_deployment_gas_price(&self) -> u64 {
        self.vm_config.min_deployment_gas_price
    }

    fn min_gas_limit(&self) -> u64 {
        self.min_gas_limit
    }

    fn min_deploy_points(&self) -> u64 {
        self.vm_config.min_deploy_points
    }
}

fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + std::hash::Hash,
{
    let mut uniq = std::collections::HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

impl Rusk {
    fn query_provisioners(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> anyhow::Result<Provisioners> {
        info!("Received get_provisioners request");
        let provisioners = self
            .provisioners(base_commit)
            .map_err(|e| anyhow::anyhow!("Cannot get provisioners {e}"))?
            .map(|(pk, stake)| {
                (PublicKey::new(pk.account), Self::to_stake(stake))
            });
        let mut ret = Provisioners::empty();
        for (pubkey_bls, stake) in provisioners {
            // Only include active provisioners
            if stake.value() > 0 {
                ret.add_provisioner(pubkey_bls, stake);
            }
        }

        Ok(ret)
    }

    fn query_provisioners_change(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> anyhow::Result<Vec<(PublicKey, Option<Stake>)>> {
        info!("Received get_provisioners_change request");
        Ok(self
            .last_provisioners_change(base_commit)
            .map_err(|e| {
                anyhow::anyhow!("Cannot get provisioners change: {e}")
            })?
            .into_iter()
            .map(|(pk, stake)| (PublicKey::new(pk), stake.map(Self::to_stake)))
            .collect())
    }

    fn to_stake(stake: StakeData) -> Stake {
        let stake_amount = stake.amount.unwrap_or_default();

        let value = stake_amount.value;

        Stake::new(value, stake_amount.eligibility)
    }
}

/// Check a state transition result against the block header
fn check_transition_result(
    transition_result: &StateTransitionResult,
    header: &Header,
) -> Result<(), StateTransitionError> {
    // Check state root
    if transition_result.state_root != header.state_hash {
        return Err(StateTransitionError::StateRootMismatch(
            transition_result.state_root,
            header.state_hash,
        ));
    }

    // Check event bloom
    if transition_result.event_bloom != header.event_bloom {
        return Err(StateTransitionError::EventBloomMismatch(
            Box::new(transition_result.event_bloom),
            Box::new(header.event_bloom),
        ));
    }

    Ok(())
}
