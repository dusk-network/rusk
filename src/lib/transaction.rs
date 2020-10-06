// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Phoenix transaction structure implementation.

use anyhow::Result;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use std::fs::File;
use std::io::{self, Read, Write};

/// PLONK proofs are constant size. However, since we can not get this
/// attribute from the `dusk_plonk` library, we declare it ourselves here
/// for convenience.
pub(crate) const PROOF_SIZE: usize = 1040;

const DEFAULT_PROOF_FILE: &'static str = "proof.bin";

fn read_default_proof() -> Result<Proof> {
    let mut proof_file = File::open(DEFAULT_PROOF_FILE)?;
    let mut buff = vec![];
    proof_file.read_to_end(&mut buff)?;
    let proof = Proof::from_bytes(&buff)?;
    Ok(proof)
}

/// All of the fields that make up a Phoenix transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub(crate) version: u8,
    pub(crate) tx_type: u8,
    pub(crate) payload: TransactionPayload,
}

/// The payload of a Phoenix transaction.
#[derive(Debug, PartialEq)]
pub struct TransactionPayload {
    pub(crate) anchor: BlsScalar,
    pub(crate) nullifiers: Vec<BlsScalar>,
    pub(crate) crossover: Option<Crossover>,
    pub(crate) notes: Vec<Note>,
    pub(crate) fee: Fee,
    pub(crate) spending_proof: Proof,
    pub(crate) call_data: Vec<u8>,
}

impl Clone for TransactionPayload {
    fn clone(&self) -> Self {
        let new_proof = Proof::from_bytes(&self.spending_proof.to_bytes())
            .expect("directly converting a valid proof should never fail");

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
            tx_type: 0,
            payload: TransactionPayload::default(),
        }
    }
}

impl Default for TransactionPayload {
    fn default() -> Self {
        TransactionPayload {
            anchor: BlsScalar::zero(),
            nullifiers: vec![],
            crossover: None,
            notes: vec![],
            fee: Fee::default(),
            // NOTE: we are unwrapping here, but this should never fail,
            // since it is a pre-generated proof which is shown to be correct.
            spending_proof: read_default_proof()
                .expect("Decoding default proof failed"),
            call_data: vec![],
        }
    }
}

impl Read for Transaction {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        n += buf.write(&[self.version])?;
        n += buf.write(&[self.tx_type])?;
        n += self.payload.read(&mut buf[n..])?;

        Ok(n)
    }
}

impl Write for Transaction {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_byte = [0u8; 1];

        n += buf.read(&mut one_byte)?;
        self.version = one_byte[0];

        n += buf.read(&mut one_byte)?;
        self.tx_type = one_byte[0];

        n += self.payload.write(&buf[n..])?;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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

        let crossover_present = self.crossover.is_some() as u8;
        n += (&mut buf[n..]).write(&[crossover_present])?;
        if crossover_present != 0 {
            n += self.crossover.unwrap().read(&mut buf[n..])?;
        }
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

        let proof_bytes = self.spending_proof.to_bytes();
        n += (&mut buf[n..]).write(&proof_bytes)?;

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

        n += (&buf[n..]).read(&mut one_byte)?;
        if one_byte[0] != 0 {
            let mut crossover = Crossover::default();
            n += crossover.write(&buf[n..])?;
            self.crossover = Some(crossover);
        }

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

        let mut proof_data = vec![0u8; PROOF_SIZE];
        n += (&buf[n..]).read(&mut proof_data)?;
        let proof = Proof::from_bytes(&proof_data).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize proof",
            )
        })?;
        self.spending_proof = proof;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::rusk_proto;
    use dusk_pki::PublicSpendKey;
    use dusk_plonk::bls12_381::Scalar as BlsScalar;
    use dusk_plonk::jubjub::{
        AffinePoint as JubJubAffine, ExtendedPoint as JubJubExtended,
        Fr as JubJubScalar, GENERATOR_EXTENDED,
    };
    use phoenix_core::{Note, NoteType};
    use std::convert::TryInto;
    use std::io::{Read, Write};

    fn deterministic_note() -> Note {
        let s = JubJubScalar::from(100 as u64);
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::from(200 as u64);
        let b = GENERATOR_EXTENDED * s;

        let pk = PublicSpendKey::new(a, b);

        let r = JubJubScalar::from(500 as u64);
        let nonce = JubJubScalar::from(800 as u64);
        let value = 1000;
        let blinding_factor = JubJubScalar::from(1200 as u64);

        Note::deterministic(
            NoteType::Obfuscated,
            &r,
            nonce,
            &pk,
            value,
            blinding_factor,
        )
    }

    fn deterministic_fee() -> Fee {
        let gas_limit: u64 = 21000;
        let gas_price: u64 = 500;

        let s = JubJubScalar::from(300 as u64);
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::from(400 as u64);
        let b = GENERATOR_EXTENDED * s;

        let psk = PublicSpendKey::new(a, b);

        Fee::new(gas_limit, gas_price, &psk)
    }

    fn deterministic_crossover() -> Crossover {
        let note = deterministic_note();
        let (_, crossover): (Fee, Crossover) = note.try_into().unwrap();
        crossover
    }

    fn deterministic_tx() -> Transaction {
        // Create a transaction with deterministic fields
        let mut tx = Transaction::default();

        tx.payload.nullifiers.push(BlsScalar::one());
        tx.payload.notes.push(deterministic_note());
        tx.payload.fee = deterministic_fee();
        tx.payload.crossover = Some(deterministic_crossover());
        tx.payload.call_data = vec![10u8; 250];
        tx
    }

    #[test]
    fn transaction_encode_decode() {
        let tx = deterministic_tx();
        let pbuf_tx: rusk_proto::Transaction = tx.clone().try_into().unwrap();
        let decoded_tx: Transaction = (&pbuf_tx).try_into().unwrap();

        assert_eq!(tx, decoded_tx);
    }

    #[test]
    fn transaction_read_write() {
        let mut tx = deterministic_tx();

        let mut buf = [0u8; 4096];
        tx.read(&mut buf).unwrap();

        let mut decoded_tx = Transaction::default();
        decoded_tx.write(&mut buf).unwrap();

        assert_eq!(tx, decoded_tx);
    }
}
