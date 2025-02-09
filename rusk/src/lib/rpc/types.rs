// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};
use yerpc::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AccountInfo {
    pub balance: u64,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct BlockInfo {
    pub block_hash: String,
    pub transactions: Vec<String>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TransactionInfo {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub status: String,
}
