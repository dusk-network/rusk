// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use juniper::{graphql_object, GraphQLObject};

#[graphql_object]
impl Transaction {
    pub fn raw(&self) -> String {
        hex::encode(self.inner.to_var_bytes())
    }

    pub fn id(&self) -> String {
        hex::encode(self.hash())
    }

    pub fn gas_limit(&self) -> f64 {
        self.inner.fee().gas_limit as f64
    }

    pub fn gas_price(&self) -> f64 {
        self.inner.fee().gas_price as f64
    }

    pub fn call_data(&self) -> Option<CallData> {
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
#[derive(GraphQLObject)]
struct CallData {
    contract_id: String,
    fn_name: String,
    data: String,
}

#[graphql_object]
impl SpentTransaction {
    pub fn err(&self) -> &Option<String> {
        &self.err
    }

    pub fn tx(&self) -> &Transaction {
        &self.inner
    }

    pub fn spent(&self) -> f64 {
        self.gas_spent as f64
    }
}

#[graphql_object]
impl Header {
    pub fn height(&self) -> f64 {
        self.height as f64
    }

    pub fn prev_block_hash(&self) -> String {
        hex::encode(self.prev_block_hash)
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp as f64
    }

    pub fn hash(&self) -> String {
        hex::encode(self.hash)
    }

    pub fn generator_bls_pubkey(&self) -> String {
        bs58::encode(self.generator_bls_pubkey.0).into_string()
    }
    pub fn tx_root(&self) -> String {
        hex::encode(self.txroot)
    }
    pub fn gas_limit(&self) -> f64 {
        self.gas_limit as f64
    }

    pub fn iteration(&self) -> i32 {
        self.iteration as i32
    }
}
