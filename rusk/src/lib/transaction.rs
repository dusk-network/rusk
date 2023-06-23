// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phoenix_core::Transaction;
use rusk_abi::ContractError;

/// The payload for a transfer transaction.
///
/// Transfer transactions are the main type of transaction in the network.
/// They can be used to transfer funds, call contracts, and even both at the
/// same time.
pub struct SpentTransaction(
    pub Transaction,
    pub u64,
    pub Option<ContractError>,
);

impl SpentTransaction {
    pub fn into_inner(self) -> (Transaction, u64, Option<ContractError>) {
        (self.0, self.1, self.2)
    }
}
