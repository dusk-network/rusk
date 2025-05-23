// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Account Models
//!
//! This module contains models related to account information used in JSON-RPC
//! responses, primarily the [`AccountInfo`] struct.

use serde::{Deserialize, Serialize};

/// Represents basic account information: nonce and balance.
///
/// This struct is typically returned by JSON-RPC methods that query the state
/// of a user account on the blockchain.
/// It mirrors the structure of `dusk_core::transfer::moonlight::AccountData`
/// for use in the JSON-RPC layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
    /// The account's current nonce.
    /// This is a counter used to prevent transaction replay attacks. Each
    /// transaction from an account must have a unique, sequential nonce.
    /// Serialized as a standard number.
    pub nonce: u64,
    /// The account's current balance.
    /// Represented in atomic units (e.g., 1 DUSK = 10^8 atomic units).
    /// Serialized as a standard number.
    pub balance: u64,
}

/// Converts the core `AccountData` type into the JSON-RPC `AccountInfo` model.
impl From<dusk_core::transfer::moonlight::AccountData> for AccountInfo {
    fn from(data: dusk_core::transfer::moonlight::AccountData) -> Self {
        AccountInfo {
            nonce: data.nonce,
            balance: data.balance,
        }
    }
}
