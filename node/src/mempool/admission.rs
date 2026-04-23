// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Mempool admission checks for canonical transactions.

use std::sync::Arc;

use anyhow::anyhow;
use node_data::ledger::{
    CanonicalTransaction, Header, LedgerTransaction, SpendingId,
};
use tokio::sync::RwLock;

use super::{
    TxAcceptanceError, check_supported_ingress_tx_format,
    should_replace_conflicting_tx,
    check_tx_serialization,
};
use crate::database::{self, Ledger, Mempool, Persist};
use crate::vm::{self, PreverificationResult};

/// Facts derived from a canonical transaction for one admission flow.
#[derive(Debug, Clone)]
pub(crate) struct AdmissionFacts {
    pub(crate) tx_id: [u8; 32],
    pub(crate) tx_size: usize,
    pub(crate) spend_ids: Vec<SpendingId>,
}

/// Result of an admission check before mutating mempool state.
#[derive(Debug)]
pub(crate) struct AdmissionCheck {
    pub(crate) facts: AdmissionFacts,
    pub(crate) tx_to_delete: Option<[u8; 32]>,
}

/// Transaction admission checks used before inserting a transaction into the
/// mempool.
pub(crate) struct TxAdmission<'a, DB, VM> {
    db: &'a Arc<RwLock<DB>>,
    vm: &'a Arc<RwLock<VM>>,
    max_mempool_txn_count: usize,
}

impl<'a, DB, VM> TxAdmission<'a, DB, VM>
where
    DB: database::DB,
    VM: vm::VMExecution,
{
    async fn load_tip_height(&self) -> Result<u64, TxAcceptanceError> {
        let tip_height = self
            .db
            .read()
            .await
            .view(|db| db.latest_block())
            .map_err(|e| {
                anyhow!("Cannot get tip block height from the database: {e}")
            })?
            .header
            .height;

        Ok(tip_height)
    }

    pub(crate) fn new(
        db: &'a Arc<RwLock<DB>>,
        vm: &'a Arc<RwLock<VM>>,
        max_mempool_txn_count: usize,
    ) -> Self {
        Self {
            db,
            vm,
            max_mempool_txn_count,
        }
    }

    pub(crate) async fn check(
        &self,
        tx: &CanonicalTransaction,
    ) -> Result<AdmissionCheck, TxAcceptanceError> {
        let tip_height = self.load_tip_height().await?;
        self.check_with_tip(tx, tip_height).await
    }

    pub(crate) async fn check_with_tip(
        &self,
        tx: &CanonicalTransaction,
        tip_height: u64,
    ) -> Result<AdmissionCheck, TxAcceptanceError> {
        let protocol = tx.protocol();
        let envelope = LedgerTransaction::from(tx.clone());
        let facts = AdmissionFacts {
            tx_id: tx.id(),
            tx_size: envelope.size(),
            spend_ids: tx.to_spend_ids(),
        };

        // We consider the maximum transaction size as the total available block
        // space minus the minimum size of the block header.
        let min_header_size = Header::default().size();
        let max_tx_size =
            dusk_consensus::config::MAX_BLOCK_SIZE - min_header_size;
        if facts.tx_size > max_tx_size {
            return Err(TxAcceptanceError::MaxSizeExceeded(facts.tx_size));
        }

        check_tx_serialization(protocol)?;

        if tx.gas_price() < 1 {
            return Err(TxAcceptanceError::GasPriceTooLow(1));
        }
        check_supported_ingress_tx_format(tx)?;

        {
            let vm = self.vm.read().await;

            let disable_wasm_32 = vm.wasm32_disabled(tip_height);
            let disable_wasm_64 = vm.wasm64_disabled(tip_height);
            let disable_3rd_party = vm.third_party_disabled(tip_height);

            if protocol.deploy().is_some() {
                match (disable_wasm_32, disable_wasm_64) {
                    (true, true) => Err(TxAcceptanceError::Generic(anyhow!(
                        "contract deployment is not enabled in the VM"
                    ))),
                    _ => Ok(()),
                }?;
            }

            if disable_3rd_party
                && let Some(call) = protocol.call()
                && call.contract != dusk_core::transfer::TRANSFER_CONTRACT
                && call.contract != dusk_core::stake::STAKE_CONTRACT
            {
                Err(TxAcceptanceError::Generic(anyhow!(
                    "3rd party contracts are not enabled in the VM"
                )))?;
            }

            protocol.phoenix_fee_check()?;

            if vm.phoenix_refund_check_active(tip_height) {
                protocol.phoenix_refund_check()?;
            }

            if protocol.deploy().is_some() {
                protocol.deploy_check(
                    vm.gas_per_deploy_byte(),
                    vm.min_deployment_gas_price(),
                    vm.min_deploy_points(),
                )?;
            }

            if protocol.blob().is_some() {
                if !vm.blob_active(tip_height) {
                    return Err(TxAcceptanceError::Generic(anyhow!(
                        "blobs are not enabled in the VM"
                    )));
                }

                protocol.blob_check(vm.gas_per_blob())?;
                dusk_consensus::validate_blob_sidecars(&envelope)?;
            }

            let min_gas_limit = vm.min_gas_limit();
            if protocol.gas_limit() < min_gas_limit {
                return Err(TxAcceptanceError::GasLimitTooLow(min_gas_limit));
            }
        }

        let tx_to_delete = self.db.read().await.view(|view| {
            if view.mempool_tx_exists(facts.tx_id)? {
                return Err(TxAcceptanceError::AlreadyExistsInMempool);
            }

            if view.ledger_tx_exists(&facts.tx_id)? {
                return Err(TxAcceptanceError::AlreadyExistsInLedger);
            }

            let txs_count = view.mempool_txs_count();
            if txs_count >= self.max_mempool_txn_count {
                let (lowest_price, to_delete) = view
                    .mempool_txs_ids_sorted_by_low_fee()
                    .next()
                    .ok_or(anyhow!("Cannot get lowest fee tx"))?;

                if tx.gas_price() < lowest_price {
                    Err(TxAcceptanceError::MaxTxnCountExceeded(
                        self.max_mempool_txn_count,
                    ))
                } else {
                    Ok(Some(to_delete))
                }
            } else {
                Ok(None)
            }
        })?;

        let preverification_data = self
            .vm
            .read()
            .await
            .preverify(tx, tip_height)
            .map_err(|e| {
                TxAcceptanceError::VerificationFailed(format!("{e}"))
            })?;

        if let PreverificationResult::FutureNonce {
            account,
            ref state,
            nonce_used,
        } = preverification_data
        {
            self.db.read().await.view(|db| {
                for nonce in state.nonce + 1..nonce_used {
                    let spending_id = SpendingId::AccountNonce(account, nonce);
                    if db
                        .mempool_txs_by_spendable_ids(&[spending_id])
                        .is_empty()
                    {
                        return Err(
                            TxAcceptanceError::MissingIntermediateNonce(nonce),
                        );
                    }
                }
                Ok(())
            })?;
        }

        Ok(AdmissionCheck {
            facts,
            tx_to_delete,
        })
    }
}

/// Applies a successful admission result to mempool state.
pub(crate) fn apply_mempool_admission<'a>(
    db: &mut impl Persist,
    tx: &'a LedgerTransaction,
    facts: &AdmissionFacts,
    tx_to_delete: Option<[u8; 32]>,
    timestamp: u64,
) -> Result<Vec<node_data::events::TransactionEvent<'a>>, TxAcceptanceError> {
    let mut events = vec![];
    let mut replaced = false;

    for m_tx_id in db.mempool_txs_by_spendable_ids(&facts.spend_ids) {
        if let Some(m_tx) = db.mempool_tx(m_tx_id)? {
            if should_replace_conflicting_tx(&m_tx, tx) {
                for deleted in db.delete_mempool_tx(m_tx_id, false)? {
                    events.push(node_data::events::TransactionEvent::Removed(
                        deleted,
                    ));
                    replaced = true;
                }
            } else {
                return Err(TxAcceptanceError::SpendIdExistsInMempool);
            }
        }
    }

    events.push(node_data::events::TransactionEvent::Included(tx));

    if !replaced && let Some(to_delete) = tx_to_delete {
        for deleted in db.delete_mempool_tx(to_delete, true)? {
            events.push(node_data::events::TransactionEvent::Removed(deleted));
        }
    }

    db.store_mempool_tx(tx, timestamp)?;

    Ok(events)
}
