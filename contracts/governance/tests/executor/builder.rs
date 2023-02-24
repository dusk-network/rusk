// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::PublicKey;
use governance_contract::Transfer;

const DUMMY_TS: u64 = 946681200000; // Dummy timestamp representing 01/01/2000

pub struct TransferBuilder(Transfer);

impl TransferBuilder {
    pub fn to(mut self, account: PublicKey) -> Self {
        self.0.to = Some(account);
        self
    }

    pub fn from(mut self, account: PublicKey) -> Self {
        self.0.from = Some(account);
        self
    }
}

pub fn transfer(amount: u64) -> TransferBuilder {
    TransferBuilder(Transfer {
        from: None,
        to: None,
        amount,
        timestamp: DUMMY_TS,
    })
}

pub fn withdraw(amount: u64) -> TransferBuilder {
    transfer(amount)
}

pub fn deposit(amount: u64) -> TransferBuilder {
    transfer(amount)
}

impl From<TransferBuilder> for Transfer {
    fn from(builder: TransferBuilder) -> Transfer {
        builder.0
    }
}
