// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use async_graphql::{Object, SimpleObject};

#[Object]
impl Block {
    #[graphql(name = "header")]
    pub async fn gql_header(&self) -> &Header {
        &self.header
    }
    #[graphql(name = "transactions")]
    pub async fn gql_txs(&self) -> &Vec<Transaction> {
        &self.txs
    }
}

#[Object]
impl Transaction {
    pub async fn raw(&self) -> String {
        hex::encode(self.inner.to_var_bytes())
    }

    pub async fn id(&self) -> String {
        hex::encode(self.hash())
    }

    pub async fn gas_limit(&self) -> u64 {
        self.inner.fee().gas_limit
    }

    #[graphql(name = "gasPrice")]
    pub async fn gql_gas_price(&self) -> u64 {
        self.inner.fee().gas_price
    }

    pub async fn call_data(&self) -> Option<CallData> {
        self.inner
            .call
            .as_ref()
            .map(|(contract_id, fn_name, data)| CallData {
                contract_id: hex::encode(contract_id),
                fn_name: fn_name.into(),
                data: hex::encode(data),
            })
    }
}
#[derive(SimpleObject)]
pub struct CallData {
    contract_id: String,
    fn_name: String,
    data: String,
}

#[Object]
impl SpentTransaction {
    pub async fn err(&self) -> &Option<String> {
        &self.err
    }

    pub async fn tx(&self) -> &Transaction {
        &self.inner
    }

    #[graphql(name = "gasSpent")]
    pub async fn gql_gas_spent(&self) -> u64 {
        self.gas_spent
    }
}

#[Object]
impl Header {
    pub async fn height(&self) -> u64 {
        self.height
    }

    pub async fn prev_block_hash(&self) -> String {
        hex::encode(self.prev_block_hash)
    }

    pub async fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub async fn hash(&self) -> String {
        hex::encode(self.hash)
    }

    pub async fn generator_bls_pubkey(&self) -> String {
        bs58::encode(self.generator_bls_pubkey.0).into_string()
    }
    pub async fn tx_root(&self) -> String {
        hex::encode(self.txroot)
    }
    pub async fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    pub async fn iteration(&self) -> u8 {
        self.iteration
    }
}
