// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::node::Rusk;
use crate::rpc::types::{AccountInfo, BlockInfo, TransactionInfo};
use anyhow::{anyhow, Result};
use bs58;
use dusk_bytes::DeserializableSlice;
use dusk_core::signatures::bls::PublicKey;
use std::sync::Arc;
use yerpc::rpc;

#[derive(Clone)]
pub struct Api {
    rusk: Arc<Rusk>,
}

impl Api {
    pub fn new(rusk: Arc<Rusk>) -> Self {
        Self { rusk }
    }

    /// Helper function to fetch account data for a given address.
    fn get_account_data(&self, address: &str) -> Result<AccountInfo> {
        let pk_bytes = bs58::decode(address)
            .into_vec()
            .map_err(|_| anyhow!("Invalid bs58 account"))?;

        let pk = PublicKey::from_slice(&pk_bytes)
            .map_err(|_| anyhow!("Invalid BLS account"))?;

        let account = self
            .rusk
            .account(&pk)
            .map_err(|e| anyhow!("Cannot query the state: {e:?}"))?;

        Ok(AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
        })
    }
}

#[rpc(all_positional = true, openrpc_outdir = "./")]
impl Api {
    /// Retrieves the current block height.
    pub async fn get_block_height(&self) -> Result<u64> {
        // TODO: Implement logic to fetch block height
        Ok(123456)
    }

    /// Retrieves the balance and nonce of a public account given its address.
    pub async fn get_account(&self, address: String) -> Result<AccountInfo> {
        self.get_account_data(&address)
    }

    /// Retrieves the balance of a public account given its address.
    pub async fn get_balance(&self, address: String) -> Result<u64> {
        self.get_account_data(&address)
            .map(|account| account.balance)
    }

    /// Retrieves block information given a block hash.
    pub async fn get_block(&self, block_hash: String) -> Result<BlockInfo> {
        // TODO: Implement logic to fetch block data
        Ok(BlockInfo {
            block_hash,
            transactions: vec!["tx1".to_string(), "tx2".to_string()],
            timestamp: 1617181723,
        })
    }

    /// Retrieves transaction details given a transaction hash.
    pub async fn get_transaction(
        &self,
        tx_hash: String,
    ) -> Result<TransactionInfo> {
        // TODO: Implement logic to fetch transaction data
        Ok(TransactionInfo {
            tx_hash,
            from: "address1".to_string(),
            to: "address2".to_string(),
            amount: 500,
            status: "confirmed".to_string(),
        })
    }
}
