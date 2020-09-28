// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Phoenix transaction structure implementation.

use crate::tx::{Crossover, Fee};
use dusk_pki::StealthAddress;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::proof_system::Proof;
use phoenix_core::note::Note;
use phoenix_core::Error;
use std::convert::TryFrom;

/// Type identifiers for Phoenix transactions.
#[derive(Debug, Copy, Clone)]
pub enum TxType {
    Transfer,
    Distribute,
    WithdrawFees,
    Bid,
    Stake,
    Slash,
    WithdrawStake,
    WithdrawBid,
}

impl From<TxType> for u32 {
    fn from(value: TxType) -> u32 {
        match value {
            TxType::Transfer => 0,
            TxType::Distribute => 1,
            TxType::WithdrawFees => 2,
            TxType::Bid => 3,
            TxType::Stake => 4,
            TxType::Slash => 5,
            TxType::WithdrawStake => 6,
            TxType::WithdrawBid => 7,
        }
    }
}

impl TryFrom<u32> for TxType {
    type Error = Error;

    fn try_from(value: u32) -> Result<TxType, Error> {
        match value {
            0 => Ok(TxType::Transfer),
            1 => Ok(TxType::Distribute),
            2 => Ok(TxType::WithdrawFees),
            3 => Ok(TxType::Bid),
            4 => Ok(TxType::Stake),
            5 => Ok(TxType::Slash),
            6 => Ok(TxType::WithdrawStake),
            7 => Ok(TxType::WithdrawBid),
            n => Err(Error::InvalidNoteType(n as u8)),
        }
    }
}

/// All of the fields that make up a Phoenix transaction.
#[derive(Debug)]
pub struct Transaction {
    version: u8,
    tx_type: TxType,
    payload: TransactionPayload,
}

/// The payload of a Phoenix transaction.
#[derive(Debug)]
pub struct TransactionPayload {
    anchor: BlsScalar,
    nullifiers: Vec<BlsScalar>,
    crossover: Crossover,
    notes: Vec<Note>,
    fee: Fee,
    spending_proof: Option<Proof>,
    call_data: Vec<u8>,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            version: 0,
            tx_type: TxType::Transfer,
            payload: TransactionPayload::default(),
        }
    }
}

impl Default for TransactionPayload {
    fn default() -> Self {
        TransactionPayload {
            anchor: BlsScalar::zero(),
            nullifiers: vec![],
            crossover: Crossover::default(),
            notes: vec![],
            fee: Fee::default(),
            spending_proof: None,
            call_data: vec![],
        }
    }
}

impl Transaction {
    /// Create a new transaction, giving all of the parameters up front.
    /// This is mostly used for deserialization from GRPC.
    pub fn new(
        version: u8,
        tx_type: TxType,
        payload: TransactionPayload,
    ) -> Self {
        Transaction {
            version,
            tx_type,
            payload,
        }
    }

    /// Set the transaction type.
    pub fn set_type(&mut self, tx_type: TxType) {
        self.tx_type = tx_type;
    }

    /// Set the fee note on the transcation.
    /// The `address` is supposed to be the wallet to which the
    /// leftover gas will be refunded.
    pub fn set_fee(
        &mut self,
        gas_limit: u64,
        gas_price: u64,
        address: StealthAddress,
    ) {
        let fee = Fee::new(gas_limit, gas_price, address);
        self.payload.fee = fee;
    }

    /// Get the transaction version.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Get the transaction type.
    pub fn tx_type(&self) -> TxType {
        self.tx_type
    }

    /// Get the transaction payload.
    pub fn payload(&self) -> &TransactionPayload {
        &self.payload
    }
}

impl TransactionPayload {
    /// Create a new transaction payload, giving all of the parameters up front.
    /// This is mostly used for deserialization from GRPC.
    pub fn new(
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        crossover: Crossover,
        notes: Vec<Note>,
        fee: Fee,
        spending_proof: Option<Proof>,
        call_data: Vec<u8>,
    ) -> Self {
        TransactionPayload {
            anchor,
            nullifiers,
            crossover,
            notes,
            fee,
            spending_proof,
            call_data,
        }
    }

    /// Set the anchor on the transaction.
    pub fn set_anchor(&mut self, anchor: BlsScalar) {
        self.anchor = anchor;
    }

    /// Add a nullifier to the transaction.
    pub fn add_nullifier(&mut self, nullifier: BlsScalar) {
        self.nullifiers.push(nullifier);
    }

    /// Add a note to the transaction.
    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    /// Set the call data for the transaction
    pub fn set_call_data(&mut self, call_data: Vec<u8>) {
        self.call_data = call_data;
    }

    /// Get the anchor belonging to the transaction.
    pub fn anchor(&self) -> BlsScalar {
        self.anchor
    }

    /// Get the nullifiers belonging to the transaction.
    pub fn nullifiers(&self) -> &[BlsScalar] {
        &self.nullifiers
    }

    /// Get the crossover note belonging to the transaction.
    pub fn crossover(&self) -> Crossover {
        self.crossover
    }

    /// Get the notes belonging to the transaction.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Get the fee note belonging to the transaction.
    pub fn fee(&self) -> Fee {
        self.fee
    }

    /// Get the spending proof belonging to the transaction.
    pub fn spending_proof(&self) -> Option<&Proof> {
        self.spending_proof.as_ref()
    }

    /// Get the call data belonging to the transaction.
    pub fn call_data(&self) -> &[u8] {
        &self.call_data
    }
}
