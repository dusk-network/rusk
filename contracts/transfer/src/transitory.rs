// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module contains data that is transitory - i.e. is built up in the
//! context of a transaction in [`spend_and_execute`] and then discarded or
//! considered void after a call to [`refund`].
//!
//! [`spend_and_execute`]: crate::spend_and_execute
//! [`refund`]: crate::refund

use core::mem;
use core::ptr::{self, addr_of_mut};

use alloc::vec::Vec;

use execution_core::{
    signatures::bls::PublicKey as AccountPublicKey,
    transfer::{
        moonlight::Transaction as MoonlightTransaction,
        phoenix::{Note, Transaction as PhoenixTransaction},
        Transaction,
    },
    ContractId,
};

/// The state of a deposit while a transaction is executing.
pub enum Deposit {
    /// There is a deposit and its still available for pick up.
    Available {
        sender: Option<AccountPublicKey>,
        target: ContractId,
        value: u64,
    },
    /// There is a deposit and it has already been picked up.
    Taken {
        sender: Option<AccountPublicKey>,
        target: ContractId,
        value: u64,
    },
    /// There is no deposit.
    None,
}

impl Deposit {
    /// Sets the state of a deposit to be `Deposit::Taken`, if it is currently
    /// in the `Deposit::Available` state. Otherwise it does nothing.
    pub fn set_taken(&mut self) {
        let mut tmp = Deposit::None;

        mem::swap(self, &mut tmp);

        match tmp {
            Deposit::Available {
                sender,
                target,
                value,
            } => {
                *self = Deposit::Taken {
                    sender,
                    target,
                    value,
                }
            }
            _ => mem::swap(self, &mut tmp),
        }
    }
}

/// The fields kept here are transitory, in the sense that they only "live"
/// while the transaction is being executed.
pub struct OngoingTransaction {
    /// The transaction currently being executed.
    pub tx: Transaction,
    /// The deposit's current state.
    pub deposit: Deposit,
    /// The notes that have been inserted into the tree.
    pub notes: Vec<Note>,
}

static mut CURRENT_TX: Option<OngoingTransaction> = None;

/// Insert the transaction into the state.
///
/// Calling this is required to use any of the other functions in this module
/// without panicking.
///
/// After you're done, you can [`take_ongoing`] to reset everything.
pub fn put_transaction(tx: impl Into<Transaction>) {
    unsafe {
        let tx = tx.into();

        let sender = tx.moonlight_sender().copied();
        let value = tx.deposit();

        let mut deposit = Deposit::None;
        if value > 0 {
            let target = tx
                .call()
                .expect("There must be a contract when depositing funds")
                .contract;

            // When a transaction is initially inserted, any deposit is
            // available for pick up.
            deposit = Deposit::Available {
                sender,
                target,
                value,
            };
        }

        CURRENT_TX = Some(OngoingTransaction {
            tx,
            deposit,
            notes: Vec::new(),
        });
    }
}

/// Takes the ongoing transaction information, "removing" it from the state.
///
/// [`put_transaction`] must be called after this to perform any other action
/// with the functions in this module.
pub fn take_ongoing() -> OngoingTransaction {
    unsafe {
        let mut tmp = None;
        ptr::swap(&mut tmp, addr_of_mut!(CURRENT_TX));
        tmp.expect("There must be an ongoing transaction")
    }
}

/// Push the note into the ongoing cache.
pub fn push_note(note: Note) {
    unsafe {
        let notes = &mut CURRENT_TX
            .as_mut()
            .expect("There must be an ongoing transaction")
            .notes;
        notes.push(note);
    }
}

/// Get a reference of the current ongoing transaction.
pub fn transaction() -> &'static Transaction {
    unsafe {
        &CURRENT_TX
            .as_ref()
            .expect("There must be an ongoing transaction")
            .tx
    }
}

/// Get a reference of the current ongoing transaction, assuming it's Moonlight.
pub fn moonlight_transaction() -> &'static MoonlightTransaction {
    match transaction() {
        Transaction::Moonlight(ref tx) => tx,
        _ => panic!("Expected Moonlight TX, found Phoenix"),
    }
}

/// Get a reference of the current ongoing transaction, assuming it's Phoenix.
pub fn phoenix_transaction() -> &'static PhoenixTransaction {
    match transaction() {
        Transaction::Phoenix(ref tx) => tx,
        _ => panic!("Expected Phoenix TX, found Moonlight"),
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
