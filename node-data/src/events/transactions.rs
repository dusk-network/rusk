// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::ledger::{Hash, SpentTransaction, Transaction};

#[derive(Clone, Debug)]
pub enum TransactionEvent<'t> {
    Removed(Hash),
    Included(&'t Transaction),
    Executed(&'t SpentTransaction),
}

impl EventSource for TransactionEvent<'_> {
    const COMPONENT: &'static str = "transactions";

    fn topic(&self) -> &'static str {
        match self {
            Self::Removed(_) => "removed",
            Self::Executed(_) => "executed",
            Self::Included(_) => "included",
        }
    }
    fn data(&self) -> EventData {
        EventData::None
    }
    fn entity(&self) -> String {
        let hash = match self {
            Self::Removed(hash) => *hash,
            Self::Executed(tx) => tx.inner.hash(),
            Self::Included(tx) => tx.hash(),
        };
        hex::encode(hash)
    }
}
