// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::{self, Display};

use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::Transaction;
use dusk_core::{dusk, from_dusk};

#[derive(Debug, PartialEq)]
pub(crate) struct TransactionHistory {
    pub(crate) direction: TransactionDirection,
    pub(crate) height: u64,
    pub(crate) amount: f64,
    pub(crate) fee: u64,
    pub(crate) tx: Transaction,
    pub(crate) id: String,
    pub(crate) bal_type: BalanceType,
}

impl TransactionHistory {
    pub fn header() -> String {
        format!(
            "{: ^9} | {: ^64} | {: ^8} | {: ^17} | {: ^12} | {: ^8}\n",
            "BLOCK", "TX_ID", "ACTION", "AMOUNT", "FEE", "BALANCE_TYPE"
        )
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub(crate) fn action(&self) -> &str {
        if self.tx.deploy().is_some() {
            "deploy"
        } else if self.tx.blob().is_some() {
            "blob"
        } else {
            match self.tx.call() {
                Some(call)
                    if call.contract == STAKE_CONTRACT
                        && call.fn_name == "withdraw" =>
                {
                    "claim-rewards"
                }
                Some(call) => &call.fn_name,
                None => "transfer",
            }
        }
    }

    /// Compact single-line string for TUI list display (no trailing newline).
    pub fn tui_line(&self) -> String {
        let amount_dusk = self.amount / dusk(1.0) as f64;
        let arrow = match self.direction {
            TransactionDirection::In => "\u{2193}",
            TransactionDirection::Out => "\u{2191}",
        };
        let action = self.action();
        let bal = match self.bal_type {
            BalanceType::Shielded => "shield",
            BalanceType::Public => "public",
        };
        format!(
            "{arrow} {action:<14} {:>+11.4} DUSK   blk {:<9}  {bal}",
            amount_dusk, self.height
        )
    }
}

impl Display for TransactionHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dusk = self.amount / dusk(1.0) as f64;
        let action = self.action();

        let fee = match self.direction {
            TransactionDirection::In => format!("{: >12.9}", ""),
            TransactionDirection::Out => {
                let fee = from_dusk(self.fee);
                format!("{: >12.9}", -fee)
            }
        };

        let tx_id = &self.id;
        let height = self.height;
        let bal_type = &self.bal_type;

        writeln!(
            f,
            "{height: >9} | {tx_id} | {action: ^8} | {dusk: >+17.9} | {fee} | {bal_type}",
        )
    }
}

#[derive(PartialEq, Debug)]
pub(crate) enum TransactionDirection {
    In,
    Out,
}

#[derive(PartialEq, Debug)]
pub(crate) enum BalanceType {
    Shielded,
    Public,
}

impl Display for BalanceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shielded => write!(f, "shielded"),
            Self::Public => write!(f, "public"),
        }
    }
}
