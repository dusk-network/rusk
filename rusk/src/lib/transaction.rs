// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Phoenix transaction structure implementation.

use anyhow::{anyhow, Result};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use std::io::{self, Read, Write};
use std::mem;

/// All of the fields that make up a Phoenix transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub(crate) version: u8,
    pub(crate) tx_type: u8,
    pub(crate) payload: TransactionPayload,
}

impl Transaction {
    pub fn new(version: u8, tx_type: u8, payload: TransactionPayload) -> Self {
        Self {
            version,
            tx_type,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.version, self.tx_type];

        bytes.extend_from_slice(&self.payload.to_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 2 {
            return Err(anyhow!("The bytes are not sufficient to deserialize the version and type of the transaction!"));
        }

        let version = bytes[0];
        let tx_type = bytes[1];
        let payload = TransactionPayload::from_bytes(&bytes[2..])?;

        Ok(Self::new(version, tx_type, payload))
    }
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

impl TransactionPayload {
    pub fn new(
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        fee: Fee,
        spending_proof: Proof,
        call_data: Vec<u8>,
    ) -> Self {
        Self {
            anchor,
            nullifiers,
            crossover,
            notes,
            fee,
            spending_proof,
            call_data,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend_from_slice(&self.anchor.to_bytes());

        bytes.extend_from_slice(&self.nullifiers.len().to_le_bytes());
        self.nullifiers
            .iter()
            .for_each(|n| bytes.extend_from_slice(&n.to_bytes()));

        bytes.push(self.crossover.is_some() as u8);
        if let Some(c) = self.crossover {
            bytes.extend_from_slice(&c.to_bytes());
        }

        bytes.extend_from_slice(&self.notes.len().to_le_bytes());
        self.notes
            .iter()
            .for_each(|n| bytes.extend_from_slice(&n.to_bytes()));

        bytes.extend_from_slice(&self.fee.to_bytes());

        bytes.extend_from_slice(&self.spending_proof.to_bytes());

        bytes.extend_from_slice(&self.call_data.len().to_le_bytes());
        bytes.extend_from_slice(self.call_data.as_slice());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        fn deser_scalar(bytes: &[u8]) -> Result<(&[u8], BlsScalar)> {
            if bytes.len() < 32 {
                return Err(anyhow!(
                    "Not enough bytes to deserialize a scalar!"
                ));
            }

            let mut s = [0u8; 32];
            s.copy_from_slice(&bytes[..32]);
            let s: Option<BlsScalar> = BlsScalar::from_bytes(&s)
                .map_err(|e| anyhow!("{:?}", e))?
                .into();
            let s =
                s.ok_or_else(|| anyhow!("Failed to deserialize a scalar!"))?;

            if bytes.len() > 32 {
                Ok((&bytes[32..], s))
            } else {
                Ok((&[], s))
            }
        }

        fn deser_usize(bytes: &[u8]) -> Result<(&[u8], usize)> {
            let mut u = usize::min_value().to_le_bytes();
            let l = u.len();

            if bytes.len() < l {
                return Err(anyhow!(
                    "Not enough bytes to deserialize an usize!"
                ));
            }

            u.copy_from_slice(&bytes[..l]);
            let u = usize::from_le_bytes(u);

            if bytes.len() > l {
                Ok((&bytes[l..], u))
            } else {
                Ok((&[], u))
            }
        }

        fn deser_bool(bytes: &[u8]) -> Result<(&[u8], bool)> {
            if bytes.is_empty() {
                return Err(anyhow!("Not enough bytes to deserialize an u8!"));
            }

            let b = bytes[0] != 0;

            if bytes.len() > 1 {
                Ok((&bytes[1..], b))
            } else {
                Ok((&[], b))
            }
        }

        let (bytes, anchor) = deser_scalar(bytes)?;

        let (bytes, items) = deser_usize(bytes)?;
        let mut nullifiers = Vec::with_capacity(items);
        let bytes =
            (0..items).try_fold::<_, _, Result<&[u8]>>(bytes, |bytes, _| {
                let (bytes, n) = deser_scalar(bytes)?;
                nullifiers.push(n);
                Ok(bytes)
            })?;

        let (bytes, crossover_present) = deser_bool(bytes)?;
        let (bytes, crossover) = if crossover_present {
            (
                &bytes[Crossover::SIZE..],
                Some(Crossover::from_slice(bytes).map_err(|e| {
                    anyhow!("Error deserializing crossover: {:?}", e)
                })?),
            )
        } else {
            (bytes, None)
        };

        let (bytes, items) = deser_usize(bytes)?;
        let mut notes = Vec::with_capacity(items);
        let bytes =
            (0..items).try_fold::<_, _, Result<&[u8]>>(bytes, |bytes, _| {
                let note = Note::from_slice(bytes).map_err(|e| {
                    anyhow!("Error deserializing note: {:?}", e)
                })?;

                notes.push(note);

                if bytes.len() > Note::SIZE {
                    Ok(&bytes[Note::SIZE..])
                } else {
                    Ok(&[])
                }
            })?;

        let fee = Fee::from_slice(bytes)
            .map_err(|e| anyhow!("Error deserializing fee: {:?}", e))?;
        let bytes = &bytes[Fee::SIZE..];

        let spending_proof = Proof::from_slice(&bytes[..Proof::SIZE])
            .map_err(|e| anyhow!("Error deserializing fee: {:?}", e))?;
        let bytes = &bytes[Proof::SIZE..];

        let (bytes, items) = deser_usize(bytes)?;
        if bytes.len() < items {
            return Err(anyhow!("Not enough bytes to deserialize call data!"));
        }
        let call_data = Vec::from(&bytes[..items]);

        Ok(Self {
            anchor,
            nullifiers,
            crossover,
            notes,
            fee,
            spending_proof,
            call_data,
        })
    }
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
        let bytes = self.to_bytes();
        let l = bytes.len();

        if buf.len() < l {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "The provided buffer is not big enough to serialize a tx payload!"));
        }

        buf[0..l].copy_from_slice(bytes.as_slice());

        Ok(l)
    }
}

impl Write for TransactionPayload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut tx = Self::from_bytes(buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize tx payload!",
            )
        })?;

        mem::swap(&mut tx, self);

        let l = self.to_bytes().len();

        Ok(l)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/*

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

        Fee::new(&mut rand::thread_rng(), gas_limit, gas_price, &psk)
    }

    fn deterministic_crossover() -> Crossover {
        let note = deterministic_note();
        let (_, crossover): (Fee, Crossover) = note.try_into().unwrap();
        crossover
    }

    fn read_default_proof() -> Result<Proof> {
        let bytes = include_bytes!("proof.bin");
        Proof::from_bytes(&bytes[..])
    }

    fn deterministic_tx_payload() -> TransactionPayload {
        let anchor = BlsScalar::default();
        let nullifiers = vec![BlsScalar::one()];
        let crossover = Some(deterministic_crossover());
        let notes = vec![deterministic_note()];
        let fee = deterministic_fee();
        let spending_proof =
            read_default_proof().expect("Failed to read proof");
        let call_data = vec![10u8; 250];

        TransactionPayload::new(
            anchor,
            nullifiers,
            crossover,
            notes,
            fee,
            spending_proof,
            call_data,
        )
    }

    fn deterministic_tx() -> Transaction {
        let payload = deterministic_tx_payload();

        Transaction::new(0, 0, payload)
    }

    #[test]
    fn transaction_encode_decode() -> Result<()> {
        let tx = deterministic_tx();
        let mut pbuf_tx = rusk_proto::Transaction::try_from(&mut tx.clone())?;
        let decoded_tx = Transaction::try_from(&mut pbuf_tx)?;

        assert_eq!(tx, decoded_tx);
        Ok(())
    }

    #[test]
    fn transaction_read_write() -> Result<()> {
        let mut tx = deterministic_tx();

        let buf = tx.to_bytes();
        let decoded_tx = Transaction::from_bytes(&buf)?;

        assert_eq!(tx, decoded_tx);
        Ok(())
    }
}

*/
