// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::{
    transfer::{
        moonlight::Transaction as MoonlightTransaction,
        phoenix::Transaction as PhoenixTransaction, Transaction,
    },
    ContractId,
};

/// The state of a deposit while a transaction is executing.
pub enum Deposit {
    /// There is a deposit and its still available for pick up.
    Available(ContractId, u64),
    /// There is a deposit and it has already been picked up.
    Taken(ContractId, u64),
    /// There is no deposit.
    None,
}

/// The fields kept here are transitory, in the sense that they only "live"
/// while the transaction is being executed.
struct CurrentTransaction {
    tx: Transaction,
    deposit: Deposit,
}

static mut CURRENT_TX: Option<CurrentTransaction> = None;

/// Get a reference to the deposit information for the currently ongoing
/// transaction.
pub fn deposit_info() -> &'static Deposit {
    unsafe {
        &CURRENT_TX
            .as_ref()
            .expect("There must be an ongoing transaction")
            .deposit
    }
}

/// Get a mutable reference to the deposit information for the currently ongoing
/// transaction.
pub fn deposit_info_mut() -> &'static mut Deposit {
    unsafe {
        &mut CURRENT_TX
            .as_mut()
            .expect("There must be an ongoing transaction")
            .deposit
    }
}

/// Insert the transaction into the state.
pub fn put_transaction(tx: impl Into<Transaction>) {
    unsafe {
        let tx = tx.into();

        let d = tx.deposit();

        let mut deposit = Deposit::None;
        if d > 0 {
            let contract = tx
                .call()
                .expect("There must be a contract when depositing funds")
                .contract;

            // When a transaction is initially inserted, any deposit is
            // available for pick up.
            deposit = Deposit::Available(contract, d);
        }

        CURRENT_TX = Some(CurrentTransaction { tx, deposit });
    }
}

pub fn unwrap_tx() -> &'static Transaction {
    unsafe {
        &CURRENT_TX
            .as_ref()
            .expect("There must be an ongoing transaction")
            .tx
    }
}

/// Unwrap ongoing transaction in the state, assuming it's Moonlight.
pub fn unwrap_moonlight_tx() -> &'static MoonlightTransaction {
    unsafe {
        let tx = &CURRENT_TX
            .as_ref()
            .expect("There must be an ongoing transaction")
            .tx;

        match tx {
            Transaction::Moonlight(ref tx) => tx,
            _ => panic!("Expected Moonlight TX, found Phoenix"),
        }
    }
}

/// Unwrap ongoing transaction in the state, assuming it's Phoenix.
pub fn unwrap_phoenix_tx() -> &'static PhoenixTransaction {
    unsafe {
        let tx = &CURRENT_TX
            .as_ref()
            .expect("There must be an ongoing transaction")
            .tx;

        match tx {
            Transaction::Phoenix(ref tx) => tx,
            _ => panic!("Expected Phoenix TX, found Moonlight"),
        }
    }
}
