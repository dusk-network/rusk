// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod query;

use tracing::info;

use dusk_bytes::DeserializableSlice;
use dusk_consensus::operations::{CallParams, VerificationOutput};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::stake::Stake;
use execution_core::{stake::StakeData, StakePublicKey};
use node::vm::VMExecution;
use node_data::ledger::{Block, SpentTransaction, Transaction};

use super::Rusk;

impl VMExecution for Rusk {
    fn execute_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        params: &CallParams,
        txs: I,
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        Vec<Transaction>,
        VerificationOutput,
    )> {
        info!("Received execute_state_transition request");

        let (txs, discarded_txs, verification_output) =
            self.execute_transactions(params, txs).map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        Ok((txs, discarded_txs, verification_output))
    }

    fn verify_state_transition(
        &self,
        blk: &Block,
    ) -> anyhow::Result<VerificationOutput> {
        info!("Received verify_state_transition request");
        let generator = blk.header().generator_bls_pubkey;
        let generator = StakePublicKey::from_slice(&generator.0)
            .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (_, verification_output) = self
            .verify_transactions(
                blk.header().height,
                blk.header().gas_limit,
                &generator,
                blk.txs(),
                &blk.header().failed_iterations.to_missed_generators()?,
            )
            .map_err(|inner| anyhow::anyhow!("Cannot verify txs: {inner}!!"))?;

        Ok(verification_output)
    }

    fn accept(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)> {
        info!("Received accept request");
        let generator = blk.header().generator_bls_pubkey;
        let generator = StakePublicKey::from_slice(&generator.0)
            .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (txs, verification_output) = self
            .accept_transactions(
                blk.header().height,
                blk.header().gas_limit,
                generator,
                blk.txs().clone(),
                Some(VerificationOutput {
                    state_root: blk.header().state_hash,
                    event_hash: blk.header().event_hash,
                }),
                &blk.header().failed_iterations.to_missed_generators()?,
            )
            .map_err(|inner| anyhow::anyhow!("Cannot accept txs: {inner}!!"))?;

        Ok((txs, verification_output))
    }

    fn finalize_state(
        &self,
        commit: [u8; 32],
        to_delete: Vec<[u8; 32]>,
    ) -> anyhow::Result<()> {
        info!("Received finalize request");
        self.finalize_state(commit, to_delete)
            .map_err(|e| anyhow::anyhow!("Cannot finalize state: {e}"))
    }

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()> {
        info!("Received preverify request");
        let tx = &tx.inner;
        let existing_nullifiers = self
            .existing_nullifiers(&tx.payload().tx_skeleton().nullifiers)
            .map_err(|e| anyhow::anyhow!("Cannot check nullifiers: {e}"))?;

        if !existing_nullifiers.is_empty() {
            let err = crate::Error::RepeatingNullifiers(existing_nullifiers);
            return Err(anyhow::anyhow!("Invalid tx: {err}"));
        }
        match crate::verifier::verify_proof(tx) {
            Ok(true) => Ok(()),
            Ok(false) => Err(anyhow::anyhow!("Invalid proof")),
            Err(e) => Err(anyhow::anyhow!("Cannot verify the proof: {e}")),
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
    ) -> anyhow::Result<Vec<(node_data::bls::PublicKey, Option<Stake>)>> {
        self.query_provisioners_change(Some(base_commit))
    }

    fn get_provisioner(
        &self,
        pk: &StakePublicKey,
    ) -> anyhow::Result<Option<Stake>> {
        let stake = self
            .provisioner(pk)
            .map_err(|e| anyhow::anyhow!("Cannot get provisioner {e}"))?
            .map(|stake| {
                let (value, eligibility) = stake.amount.unwrap_or_default();
                Stake::new(value, stake.reward, eligibility, stake.counter)
            });
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
                (node_data::bls::PublicKey::new(pk), Self::to_stake(stake))
            });
        let mut ret = Provisioners::empty();
        for (pubkey_bls, stake) in provisioners {
            ret.add_member_with_stake(pubkey_bls, stake);
        }

        Ok(ret)
    }

    fn query_provisioners_change(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> anyhow::Result<Vec<(node_data::bls::PublicKey, Option<Stake>)>> {
        info!("Received get_provisioners_change request");
        Ok(self
            .last_provisioners_change(base_commit)
            .map_err(|e| {
                anyhow::anyhow!("Cannot get provisioners change: {e}")
            })?
            .into_iter()
            .map(|(pk, stake)| {
                (
                    node_data::bls::PublicKey::new(pk),
                    stake.map(Self::to_stake),
                )
            })
            .collect())
    }

    fn to_stake(stake: StakeData) -> Stake {
        let (value, eligibility) = stake.amount.unwrap_or_default();
        Stake::new(value, stake.reward, eligibility, stake.counter)
    }
}
