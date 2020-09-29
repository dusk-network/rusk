// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

//! Phoenix transaction structure implementation.

use crate::tx::{Crossover, Fee};
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::proof_system::Proof;
use phoenix_core::note::Note;
use phoenix_core::Error;
use std::convert::{TryFrom, TryInto};
use std::io::{self, Read, Write};
use std::mem::transmute_copy;

/// Type identifiers for Phoenix transactions.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

impl Read for TxType {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        buf.write(&(u32::from(*self) as u8).to_le_bytes())
    }
}

impl Write for TxType {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;
        let mut one_byte = [0u8; 1];

        n += buf.read(&mut one_byte)?;
        *self = (one_byte[0] as u32).try_into()?;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// All of the fields that make up a Phoenix transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    version: u8,
    tx_type: TxType,
    payload: TransactionPayload,
}

/// The payload of a Phoenix transaction.
#[derive(Debug, PartialEq)]
pub struct TransactionPayload {
    anchor: BlsScalar,
    nullifiers: Vec<BlsScalar>,
    crossover: Crossover,
    notes: Vec<Note>,
    fee: Fee,
    spending_proof: Option<Proof>,
    call_data: Vec<u8>,
}

impl Clone for TransactionPayload {
    fn clone(&self) -> Self {
        let mut new_proof: Option<Proof> = None;
        if self.spending_proof().is_some() {
            unsafe {
                new_proof =
                    Some(transmute_copy(self.spending_proof().unwrap()));
            }
        }

        TransactionPayload {
            anchor: self.anchor.clone(),
            nullifiers: self.nullifiers.clone(),
            crossover: self.crossover.clone(),
            notes: self.notes.clone(),
            fee: self.fee.clone(),
            spending_proof: new_proof,
            call_data: self.call_data.clone(),
        }
    }
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

    /// Get a mutable reference to the transaction payload.
    pub fn mut_payload(&mut self) -> &mut TransactionPayload {
        &mut self.payload
    }
}

impl Read for Transaction {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        n += buf.write(&self.version.to_le_bytes())?;
        n += self.tx_type.read(&mut buf[n..])?;
        n += self.payload.read(&mut buf[n..])?;

        Ok(n)
    }
}

impl Write for Transaction {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_byte = [0u8; 1];

        n += buf.read(&mut one_byte)?;
        self.version = u8::from_le_bytes(one_byte);

        n += self.tx_type.write(&buf[n..])?;
        n += self.payload.write(&buf[n..])?;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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

    /// Set the fee note on the transcation.
    /// The `address` is supposed to be the wallet to which the
    /// leftover gas will be refunded.
    pub fn set_fee(&mut self, fee: Fee) {
        self.fee = fee;
    }

    /// Set the crossover note on the transaction.
    pub fn set_crossover(&mut self, crossover: Crossover) {
        self.crossover = crossover;
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

impl Read for TransactionPayload {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        n += (&mut buf[n..]).write(&self.anchor.to_bytes())?;
        n += (&mut buf[n..])
            .write(&(self.nullifiers.len() as u64).to_le_bytes())?;

        self.nullifiers
            .iter()
            .map(|nul| {
                (&mut buf[n..]).write(&nul.to_bytes()).and_then(|num| {
                    n += num;
                    Ok(num)
                })
            })
            .collect::<io::Result<Vec<usize>>>()?;

        n += self.crossover.read(&mut buf[n..])?;
        n += (&mut buf[n..]).write(&(self.notes.len() as u64).to_le_bytes())?;

        self.notes
            .iter_mut()
            .map(|note| {
                note.read(&mut buf[n..]).and_then(|num| {
                    n += num;
                    Ok(num)
                })
            })
            .collect::<io::Result<Vec<usize>>>()?;

        n += self.fee.read(&mut buf[n..])?;

        let proof_present = self.spending_proof.is_some() as u8;
        n += (&mut buf[n..]).write(&proof_present.to_le_bytes())?;
        if proof_present != 0 {
            let proof_bytes = self.spending_proof.as_ref().unwrap().to_bytes();
            n += (&mut buf[n..])
                .write(&(proof_bytes.len() as u64).to_le_bytes())?;
            n += (&mut buf[n..]).write(&proof_bytes)?;
        }

        n += (&mut buf[n..])
            .write(&(self.call_data.len() as u64).to_le_bytes())?;
        n += (&mut buf[n..]).write(&self.call_data)?;

        Ok(n)
    }
}

impl Write for TransactionPayload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_scalar = [0u8; 32];
        let mut one_u64 = [0u8; 8];
        let mut one_note = [0u8; 233];
        let mut one_byte = [0u8; 1];

        n += (&buf[n..]).read(&mut one_scalar)?;
        self.anchor = Option::from(BlsScalar::from_bytes(&one_scalar)).ok_or(
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize anchor",
            ),
        )?;

        n += (&buf[n..]).read(&mut one_u64)?;
        let nul_size = u64::from_le_bytes(one_u64) as usize;
        self.nullifiers = Vec::<BlsScalar>::with_capacity(nul_size);
        (0..nul_size)
            .into_iter()
            .map(|_| {
                (&buf[n..]).read(&mut one_scalar).and_then(|num| {
                    n += num;
                    self.nullifiers.push(
                        Option::from(BlsScalar::from_bytes(&one_scalar))
                            .ok_or(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Could not deserialize nullifier",
                            ))?,
                    );

                    Ok(n)
                })
            })
            .collect::<Result<Vec<usize>, io::Error>>()?;

        n += self.crossover.write(&buf[n..])?;

        n += (&buf[n..]).read(&mut one_u64)?;
        let notes_size = u64::from_le_bytes(one_u64) as usize;
        self.notes = vec![Note::default(); notes_size];
        self.notes
            .iter_mut()
            .map(|note| {
                (&buf[n..]).read(&mut one_note).and_then(|num| {
                    n += num;
                    note.write(&one_note).and_then(|_| Ok(num))
                })
            })
            .collect::<Result<Vec<usize>, io::Error>>()?;

        n += self.fee.write(&buf[n..])?;

        n += (&buf[n..]).read(&mut one_byte)?;
        if u8::from_le_bytes(one_byte) != 0 {
            n += (&buf[n..]).read(&mut one_u64)?;
            let proof_size = u64::from_le_bytes(one_u64) as usize;
            let mut proof_data = vec![0u8; proof_size];
            n += (&buf[n..]).read(&mut proof_data)?;
            let proof = Proof::from_bytes(&proof_data).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Could not deserialize proof",
                )
            })?;
            self.spending_proof = Some(proof);
        }

        n += (&buf[n..]).read(&mut one_u64)?;
        let call_data_size = u64::from_le_bytes(one_u64) as usize;
        let mut call_data = vec![0u8; call_data_size];
        n += (&buf[n..]).read(&mut call_data)?;
        self.call_data = call_data;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
