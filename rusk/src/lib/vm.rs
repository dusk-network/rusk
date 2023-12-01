// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::DeserializableSlice;
use dusk_consensus::contract_state::{CallParams, VerificationOutput};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::stake::Stake;
use node::vm::VMExecution;
use node_data::ledger::{Block, SpentTransaction, Transaction};
use tracing::info;

use crate::Rusk;

impl VMExecution for Rusk {
    fn execute_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        params: CallParams,
        txs: I,
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        Vec<Transaction>,
        VerificationOutput,
    )> {
        info!("Received execute_state_transition request");

        let (txs, discarded_txs, verification_output) = self
            .execute_transactions(
                params.round,
                params.block_gas_limit,
                params.generator_pubkey.inner(),
                txs,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        Ok((txs, discarded_txs, verification_output))
    }

    fn verify_state_transition(
        &self,
        params: &CallParams,
        txs: Vec<Transaction>,
    ) -> anyhow::Result<VerificationOutput> {
        info!("Received verify_state_transition request");

        let (_, verification_output) = self
            .verify_transactions(
                params.round,
                params.block_gas_limit,
                params.generator_pubkey.inner(),
                &txs[..],
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
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
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
            )
            .map_err(|inner| anyhow::anyhow!("Cannot accept txs: {inner}!!"))?;

        Ok((txs, verification_output))
    }

    fn finalize(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)> {
        info!("Received finalize request");
        let generator = blk.header().generator_bls_pubkey;
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (txs, state_root) = self
            .finalize_transactions(
                blk.header().height,
                blk.header().gas_limit,
                generator,
                blk.txs().clone(),
                Some(VerificationOutput {
                    state_root: blk.header().state_hash,
                    event_hash: blk.header().event_hash,
                }),
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot finalize txs: {inner}!!")
            })?;

        Ok((txs, state_root))
    }

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()> {
        info!("Received preverify request");
        let tx = &tx.inner;
        let existing_nullifiers = self
            .existing_nullifiers(&tx.nullifiers)
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

    fn get_provisioners(&self) -> anyhow::Result<Provisioners> {
        info!("Received get_provisioners request");
        let provisioners = self
            .provisioners()
            .map_err(|e| anyhow::anyhow!("Cannot get provisioners {e}"))?
            .into_iter()
            .filter_map(|(key, stake)| {
                stake.amount.map(|(value, eligibility)| {
                    let stake = Stake::new(value, stake.reward, eligibility);
                    let pubkey_bls = node_data::bls::PublicKey::new(key);
                    (pubkey_bls, stake)
                })
            });
        let mut ret = Provisioners::new();
        for (pubkey_bls, stake) in provisioners {
            ret.add_member_with_stake(pubkey_bls, stake);
        }

        Ok(ret)
    }

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.state_root())
    }

    fn revert(&self) -> anyhow::Result<[u8; 32]> {
        let state_hash = self
            .revert()
            .map_err(|inner| anyhow::anyhow!("Cannot revert: {inner}"))?;

        Ok(state_hash)
    }
}
