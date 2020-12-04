// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Phoenix transaction structure implementation.

use anyhow::Result;
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::JubJubScalar;
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use std::io::{self, Read, Write};

fn read_default_proof() -> Result<Proof> {
    let bytes = include_bytes!("proof.bin");
    Proof::from_bytes(&bytes[..])
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
            anchor: self.anchor,
            nullifiers: self.nullifiers.clone(),
            crossover: self.crossover,
            notes: self.notes.clone(),
            fee: self.fee,
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
            fee: Fee::deterministic(
                0u64,
                0u64,
                &JubJubScalar::default(),
                &PublicSpendKey::from(SecretSpendKey::new(
                    JubJubScalar::default(),
                    JubJubScalar::default(),
                )),
            ),

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

        // Version
        n += buf.write(&[self.version])?;
        // Type
        n += buf.write(&[self.tx_type])?;
        // Payload
        n += self.payload.read(&mut buf[n..])?;

        Ok(n)
    }
}

impl Write for Transaction {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_byte = [0u8; 1];

        // Version
        n += buf.read(&mut one_byte)?;
        self.version = one_byte[0];

        // Type
        n += buf.read(&mut one_byte)?;
        self.tx_type = one_byte[0];

        // Payload
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

        // Anchor
        n += (&mut buf[n..]).write(&self.anchor.to_bytes())?;
        n += (&mut buf[n..])
            .write(&(self.nullifiers.len() as u64).to_le_bytes())?;

        // Nullifiers
        self.nullifiers
            .iter()
            .map(|nul| {
                (&mut buf[n..]).write(&nul.to_bytes()).map(|num| {
                    n += num;
                    num
                })
            })
            .collect::<io::Result<Vec<usize>>>()?;

        // Crossover
        let crossover_present = self.crossover.is_some() as u8;
        n += (&mut buf[n..]).write(&[crossover_present])?;
        if crossover_present != 0 {
            buf[n..n + Crossover::serialized_size()]
                .copy_from_slice(&self.crossover.unwrap().to_bytes());
            n += Crossover::serialized_size();
        }
        n += (&mut buf[n..]).write(&(self.notes.len() as u64).to_le_bytes())?;

        // Notes
        self.notes.iter().for_each(|note| {
            buf[n..n + Note::serialized_size()]
                .copy_from_slice(&note.to_bytes());
            n += Note::serialized_size();
        });

        // Fee
        buf[n..n + Fee::serialized_size()]
            .copy_from_slice(&self.fee.to_bytes());
        n += Fee::serialized_size();

        // Proof
        let proof_bytes = self.spending_proof.to_bytes();
        n += (&mut buf[n..]).write(&proof_bytes)?;

        // Call data
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

        // Anchor
        n += (&buf[n..]).read(&mut one_scalar)?;
        self.anchor = Option::from(BlsScalar::from_bytes(&one_scalar))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Could not deserialize anchor",
                )
            })?;

        // Nullifiers
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
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "Could not deserialize nullifier",
                                )
                            })?,
                    );

                    Ok(n)
                })
            })
            .collect::<Result<Vec<usize>, io::Error>>()?;

        // Crossover
        n += (&buf[n..]).read(&mut one_byte)?;
        if one_byte[0] != 0 {
            let mut one_crossover = [0u8; Crossover::serialized_size()];
            one_crossover[..]
                .copy_from_slice(&buf[n..n + Crossover::serialized_size()]);
            self.crossover =
                Some(Crossover::from_bytes(&one_crossover).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("{:?}", e),
                    )
                })?);
            n += Crossover::serialized_size();
        }

        // Notes
        n += (&buf[n..]).read(&mut one_u64)?;
        let notes_size = u64::from_le_bytes(one_u64) as usize;
        self.notes = Vec::<Note>::with_capacity(notes_size);
        (0..notes_size)
            .into_iter()
            .map(|_| {
                (&buf[n..]).read(&mut one_note).and_then(|num| {
                    n += num;
                    self.notes.push(Note::from_bytes(&one_note).map_err(
                        |e| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("{:?}", e),
                            )
                        },
                    )?);
                    Ok(num)
                })
            })
            .collect::<Result<Vec<usize>, io::Error>>()?;

        // Fee
        let mut one_fee = [0u8; Fee::serialized_size()];
        one_fee[..].copy_from_slice(&buf[n..n + Fee::serialized_size()]);
        self.fee = Fee::from_bytes(&one_fee).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{:?}", e),
            )
        })?;
        n += Fee::serialized_size();

        // Proof
        let mut proof_data = vec![0u8; Proof::serialised_size()];
        n += (&buf[n..]).read(&mut proof_data)?;
        let proof = Proof::from_bytes(&proof_data).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize proof",
            )
        })?;
        self.spending_proof = proof;

        // Call data
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
    use dusk_plonk::bls12_381::BlsScalar;
    use dusk_plonk::jubjub::{JubJubScalar, GENERATOR_EXTENDED};
    use phoenix_core::{Note, NoteType};
    use std::convert::{TryFrom, TryInto};
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
    fn transaction_encode_decode() -> Result<()> {
        let mut tx = deterministic_tx();
        let mut pbuf_tx = rusk_proto::Transaction::try_from(&mut tx.clone())?;
        let decoded_tx = Transaction::try_from(&mut pbuf_tx)?;

        assert_eq!(tx, decoded_tx);
        Ok(())
    }

    #[test]
    fn transaction_read_write() -> Result<()> {
        let mut tx = deterministic_tx();

        let mut buf = [0u8; 4096];
        tx.read(&mut buf)?;

        let mut decoded_tx = Transaction::default();
        decoded_tx.write(&mut buf)?;

        assert_eq!(tx, decoded_tx);
        Ok(())
    }
}
