// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_consensus::{
    contract_state::CallParams,
    user::{
        provisioners::{Member, Provisioners, DUSK},
        stake::Stake,
    },
};
use node::vm::{Config, VMExecution};
use node_data::ledger::{Block, Transaction};
use rusk::Rusk;
use tracing::info;

/// Empty Placeholder for VMExecution
pub struct VMExecutionImpl {
    inner: Rusk,
}

impl VMExecutionImpl {
    pub fn new(_conf: Config, rusk: Rusk) -> Self {
        Self { inner: rusk }
    }
}

impl VMExecution for VMExecutionImpl {
    fn execute_state_transition(
        &self,
        params: &CallParams,
    ) -> anyhow::Result<(Vec<Transaction>, Vec<Transaction>, [u8; 32])> {
        info!("Received execute_state_transition request");
        let generator = params.generator_pubkey.clone();

        // Deserialize transactions, collecting failed ones in the
        // `discarded_txs`. This is then appended to with failed transactions.
        let txs = params.txs.iter().cloned().map(|t| t.inner).collect();

        let (txs, discarded_txs, state_root) = self
            .inner
            .execute_transactions(
                params.round,
                params.block_gas_limit,
                *generator.inner(),
                txs,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        let txs = txs
            .into_iter()
            .map(|tx| Transaction {
                gas_spent: Some(tx.1),
                inner: tx.0,
                err: tx.2.map(|e| format!("{e:?}")),
            })
            .collect();

        let discarded_txs = discarded_txs
            .into_iter()
            .map(|t| Transaction {
                gas_spent: None,
                inner: t,
                err: None,
            })
            .collect();

        Ok((txs, discarded_txs, state_root))
    }

    fn verify_state_transition(
        &self,
        params: &CallParams,
    ) -> anyhow::Result<[u8; 32]> {
        info!("Received verify_state_transition request");
        let generator = params.generator_pubkey.clone();

        // Deserialize transactions, collecting failed ones in the
        // `discarded_txs`. This is then appended to with failed transactions.
        let txs = params.txs.iter().cloned().map(|t| t.inner).collect();

        let (_, state_root) = self
            .inner
            .verify_transactions(
                params.round,
                params.block_gas_limit,
                *generator.inner(),
                txs,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        Ok(state_root)
    }

    fn accept(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<Transaction>, [u8; 32])> {
        info!("Received accept request");
        let generator = blk.header.generator_bls_pubkey.clone();
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        // Deserialize transactions, collecting failed ones in the
        // `discarded_txs`. This is then appended to with failed transactions.
        let txs = blk.txs.iter().cloned().map(|t| t.inner).collect();

        let (txs, state_root) = self
            .inner
            .accept_transactions(
                blk.header.height,
                blk.header.gas_limit,
                generator,
                txs,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        let txs = txs
            .into_iter()
            .map(|tx| Transaction {
                gas_spent: Some(tx.1),
                inner: tx.0,
                err: tx.2.map(|e| format!("{e:?}")),
            })
            .collect();

        Ok((txs, state_root))
    }

    fn finalize(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<Transaction>, [u8; 32])> {
        info!("Received finalize request");
        let generator = blk.header.generator_bls_pubkey.clone();
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        // Deserialize transactions, collecting failed ones in the
        // `discarded_txs`. This is then appended to with failed transactions.
        let txs = blk.txs.iter().cloned().map(|t| t.inner).collect();

        let (txs, state_root) = self
            .inner
            .finalize_transactions(
                blk.header.height,
                blk.header.gas_limit,
                generator,
                txs,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        let txs = txs
            .into_iter()
            .map(|tx| Transaction {
                gas_spent: Some(tx.1),
                inner: tx.0,
                err: tx.2.map(|e| format!("{e:?}")),
            })
            .collect();

        Ok((txs, state_root))
    }

    fn preverify(&self, _tx: &Transaction) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_provisioners(&self) -> Result<Provisioners, anyhow::Error> {
        info!("Received get_provisioners request");
        let provisioners = self
            .inner
            .provisioners()
            .map_err(|e| anyhow::anyhow!("Cannot get provisioners {e}"))?
            .into_iter()
            .filter_map(|(key, stake)| {
                stake.amount.map(|(value, eligibility)| {
                    // let raw_public_key_bls = key.to_raw_bytes().to_vec();
                    let public_key_bls = key.to_bytes().to_vec();

                    let pk = dusk_bls12_381_sign::PublicKey::from_slice(
                        &public_key_bls,
                    )
                    .expect("Provisioner data to be a valid publickey");

                    let mut m = Member::new(node_data::bls::PublicKey::new(pk));
                    m.add_stake(Stake::new(value, stake.reward, eligibility));
                    m
                    // Provisioner {
                    //     raw_public_key_bls,
                    //     public_key_bls,
                    //     stakes: vec![stake],
                    // }
                })
            });
        let mut ret = Provisioners::new();
        for p in provisioners {
            let first_stake = p
                .first_stake()
                .expect("Provisioners must have at least one stake");
            ret.add_member_with_stake(
                p.public_key().clone(),
                first_stake.clone(),
            )
        }

        Ok(ret)
    }

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.inner.state_root())
    }
}
