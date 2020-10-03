// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Phoenix transaction structure implementation.

use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use std::io::{self, Read, Write};
use std::mem::transmute_copy;

/// PLONK proofs are constant size. However, since we can not get this
/// attribute from the `dusk_plonk` library, we declare it ourselves here
/// for convenience.
pub const PROOF_SIZE: usize = 1040;

const DEFAULT_PROOF_BYTES: [u8; 1040] = [
    170, 161, 69, 162, 186, 114, 128, 191, 233, 75, 200, 123, 129, 208, 217,
    183, 186, 165, 191, 134, 80, 225, 163, 225, 93, 117, 79, 138, 235, 159, 98,
    157, 251, 55, 186, 143, 5, 73, 207, 252, 4, 138, 55, 48, 86, 43, 79, 106,
    164, 33, 201, 127, 177, 218, 94, 184, 168, 63, 232, 149, 175, 37, 92, 103,
    62, 76, 118, 188, 221, 62, 249, 207, 67, 202, 34, 1, 57, 211, 13, 238, 184,
    93, 229, 80, 81, 177, 217, 204, 34, 24, 103, 54, 173, 92, 15, 167, 169,
    166, 8, 92, 28, 129, 97, 0, 217, 87, 30, 74, 111, 60, 32, 61, 49, 80, 196,
    17, 0, 187, 114, 142, 224, 133, 139, 169, 23, 108, 3, 34, 17, 16, 117, 252,
    143, 81, 123, 57, 205, 109, 153, 1, 124, 218, 139, 122, 166, 86, 122, 244,
    102, 8, 15, 144, 223, 74, 173, 203, 70, 105, 209, 37, 228, 55, 197, 75, 78,
    10, 93, 57, 30, 231, 97, 101, 19, 22, 89, 159, 69, 169, 69, 44, 97, 119,
    157, 221, 172, 85, 209, 159, 232, 47, 25, 79, 175, 254, 179, 124, 106, 216,
    89, 186, 20, 114, 254, 246, 165, 244, 195, 200, 104, 34, 92, 109, 129, 240,
    106, 21, 166, 146, 75, 40, 194, 138, 99, 144, 5, 246, 90, 179, 174, 109,
    37, 223, 134, 196, 221, 66, 155, 206, 64, 23, 141, 1, 22, 238, 184, 73,
    125, 98, 59, 220, 65, 243, 173, 233, 73, 158, 64, 35, 124, 103, 254, 251,
    239, 39, 219, 54, 14, 248, 54, 219, 78, 58, 244, 204, 250, 112, 233, 124,
    34, 181, 78, 226, 13, 97, 136, 14, 151, 254, 128, 253, 168, 11, 142, 183,
    46, 198, 242, 93, 222, 135, 226, 66, 33, 126, 97, 46, 121, 114, 202, 254,
    108, 231, 253, 115, 209, 103, 237, 237, 195, 123, 51, 115, 230, 118, 23,
    95, 146, 190, 138, 203, 173, 32, 59, 35, 185, 208, 139, 166, 90, 110, 194,
    149, 125, 62, 200, 179, 168, 136, 148, 78, 233, 68, 91, 131, 211, 80, 127,
    123, 225, 19, 158, 136, 216, 93, 253, 13, 118, 52, 225, 71, 127, 240, 179,
    231, 248, 215, 21, 224, 28, 113, 65, 196, 35, 210, 166, 39, 207, 112, 211,
    208, 203, 223, 195, 34, 216, 3, 88, 80, 64, 148, 119, 175, 243, 56, 27, 76,
    68, 121, 224, 22, 252, 152, 220, 64, 38, 72, 24, 216, 239, 74, 81, 101, 32,
    243, 53, 162, 176, 191, 73, 197, 8, 73, 175, 119, 10, 154, 125, 49, 208,
    180, 215, 208, 193, 12, 163, 151, 155, 95, 213, 42, 239, 48, 40, 170, 61,
    253, 93, 32, 30, 63, 50, 29, 120, 197, 160, 121, 80, 65, 228, 23, 248, 227,
    82, 187, 114, 224, 26, 140, 110, 168, 141, 168, 27, 183, 166, 131, 145,
    208, 161, 193, 9, 27, 32, 23, 202, 157, 31, 14, 128, 81, 76, 203, 160, 28,
    169, 138, 90, 45, 75, 196, 56, 31, 157, 11, 217, 136, 71, 88, 102, 226, 4,
    88, 7, 246, 147, 101, 189, 150, 86, 206, 49, 176, 118, 233, 0, 234, 177,
    19, 0, 202, 65, 78, 154, 230, 17, 4, 64, 2, 161, 251, 178, 0, 53, 7, 87,
    243, 162, 156, 171, 65, 11, 231, 136, 131, 150, 251, 136, 143, 203, 30, 57,
    50, 248, 38, 212, 91, 11, 83, 216, 210, 91, 237, 43, 95, 101, 75, 120, 107,
    228, 183, 71, 107, 7, 23, 112, 201, 157, 238, 219, 214, 237, 160, 141, 214,
    38, 215, 70, 123, 135, 167, 56, 5, 44, 87, 110, 83, 138, 91, 43, 74, 82,
    38, 7, 32, 166, 71, 35, 57, 161, 124, 223, 14, 192, 67, 37, 155, 228, 54,
    212, 108, 68, 4, 86, 217, 95, 202, 47, 249, 238, 2, 170, 148, 52, 169, 196,
    9, 37, 137, 3, 203, 206, 28, 44, 191, 191, 226, 189, 225, 172, 174, 183,
    74, 148, 121, 253, 153, 2, 43, 168, 214, 196, 75, 74, 137, 34, 249, 121,
    113, 135, 121, 80, 35, 69, 61, 235, 28, 7, 201, 94, 174, 60, 248, 198, 46,
    83, 10, 44, 92, 23, 107, 214, 146, 164, 133, 241, 37, 25, 238, 61, 85, 73,
    14, 85, 228, 218, 102, 142, 183, 97, 109, 201, 23, 160, 244, 228, 81, 68,
    120, 107, 17, 178, 77, 251, 82, 103, 224, 220, 222, 234, 40, 100, 59, 113,
    133, 53, 25, 171, 88, 195, 207, 117, 13, 136, 125, 137, 15, 46, 38, 12,
    122, 227, 93, 218, 117, 98, 142, 219, 188, 248, 201, 32, 129, 91, 103, 56,
    75, 56, 207, 85, 118, 156, 116, 4, 216, 230, 151, 14, 156, 81, 119, 110,
    142, 155, 232, 115, 207, 182, 34, 118, 228, 175, 83, 6, 110, 69, 58, 51,
    191, 135, 26, 123, 17, 87, 67, 246, 72, 225, 217, 183, 100, 49, 124, 9,
    117, 18, 101, 143, 163, 125, 134, 5, 61, 206, 43, 104, 234, 22, 113, 142,
    46, 78, 137, 96, 106, 44, 170, 91, 40, 188, 246, 141, 208, 235, 147, 105,
    98, 164, 245, 245, 169, 138, 97, 157, 255, 193, 63, 221, 165, 133, 88, 29,
    217, 28, 78, 64, 3, 45, 241, 55, 156, 136, 224, 67, 218, 143, 80, 113, 96,
    43, 229, 166, 174, 185, 160, 34, 179, 162, 36, 212, 44, 4, 46, 141, 246,
    184, 73, 222, 33, 246, 228, 166, 1, 109, 119, 20, 41, 176, 57, 169, 141,
    10, 149, 25, 26, 244, 207, 176, 54, 223, 98, 23, 79, 73, 35, 142, 225, 78,
    94, 155, 175, 164, 146, 63, 55, 180, 173, 1, 66, 0, 112, 203, 1, 157, 177,
    134, 228, 164, 212, 254, 63, 188, 199, 25, 172, 248, 13, 170, 6, 212, 158,
    244, 18, 24, 93, 214, 54, 148, 71, 127, 121, 85, 144, 208, 41, 192, 1, 143,
    17, 67, 149, 130, 157, 177, 252, 121, 30, 51, 48, 218, 218, 46, 126, 11,
    247, 123, 50, 154, 188, 101, 71, 93, 82, 52, 38, 198, 57, 123, 2, 252, 179,
    132, 24, 45, 203, 103, 189, 16, 249, 9, 48,
];

/// All of the fields that make up a Phoenix transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    version: u8,
    tx_type: u8,
    payload: TransactionPayload,
}

/// The payload of a Phoenix transaction.
#[derive(Debug, PartialEq)]
pub struct TransactionPayload {
    anchor: BlsScalar,
    nullifiers: Vec<BlsScalar>,
    crossover: Option<Crossover>,
    notes: Vec<Note>,
    fee: Fee,
    spending_proof: Proof,
    call_data: Vec<u8>,
}

impl Clone for TransactionPayload {
    fn clone(&self) -> Self {
        let new_proof: Proof;
        unsafe {
            new_proof = transmute_copy(self.spending_proof());
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
            spending_proof: Proof::from_bytes(&DEFAULT_PROOF_BYTES)
                .expect("Decoding default proof failed"),
            call_data: vec![],
        }
    }
}

impl Transaction {
    /// Create a new transaction, giving all of the parameters up front.
    /// This is mostly used for deserialization from GRPC.
    pub fn new(version: u8, tx_type: u8, payload: TransactionPayload) -> Self {
        Transaction {
            version,
            tx_type,
            payload,
        }
    }

    /// Set the transaction type.
    pub fn set_type(&mut self, tx_type: u8) {
        self.tx_type = tx_type;
    }

    /// Get the transaction version.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Get the transaction type.
    pub fn tx_type(&self) -> u8 {
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
        n += buf.write(&self.tx_type.to_le_bytes())?;
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

        n += buf.read(&mut one_byte)?;
        self.tx_type = u8::from_le_bytes(one_byte);

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
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        fee: Fee,
        spending_proof: Proof,
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
        self.crossover = Some(crossover);
    }

    /// Add a nullifier to the transaction.
    pub fn add_nullifier(&mut self, nullifier: BlsScalar) {
        self.nullifiers.push(nullifier);
    }

    /// Add a note to the transaction.
    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
    }

    /// Set the proof on the transaction.
    pub fn set_proof(&mut self, proof: Proof) {
        self.spending_proof = proof;
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
    pub fn crossover(&self) -> Option<Crossover> {
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
    pub fn spending_proof(&self) -> &Proof {
        &self.spending_proof
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

        let crossover_present = self.crossover.is_some() as u8;
        n += (&mut buf[n..]).write(&crossover_present.to_le_bytes())?;
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
        if u8::from_le_bytes(one_byte) != 0 {
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
    use phoenix_core::Note;
    use rand::Rng;
    use std::convert::TryInto;
    use std::io::{Read, Write};

    fn random_note() -> Note {
        let t: u8 = rand::thread_rng().gen_range(0, 2);

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let a = JubJubExtended::from(JubJubAffine::from(a));
        let b = JubJubExtended::from(JubJubAffine::from(b));

        let pk = PublicSpendKey::new(a, b);

        let value: u64 = rand::thread_rng().gen();

        Note::new(t.try_into().unwrap(), &pk, value)
    }

    fn random_fee() -> Fee {
        let gas_limit: u64 = rand::thread_rng().gen();
        let gas_price: u64 = rand::thread_rng().gen();

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let a = JubJubExtended::from(JubJubAffine::from(a));
        let b = JubJubExtended::from(JubJubAffine::from(b));

        let psk = PublicSpendKey::new(a, b);

        Fee::new(gas_limit, gas_price, &psk)
    }

    fn random_crossover() -> Crossover {
        let s = JubJubScalar::random(&mut rand::thread_rng());
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let a = JubJubExtended::from(JubJubAffine::from(a));
        let b = JubJubExtended::from(JubJubAffine::from(b));

        let value: u64 = rand::thread_rng().gen();

        let psk = PublicSpendKey::new(a, b);
        let note = Note::obfuscated(&psk, value);
        let (_, crossover): (Fee, Crossover) = note.try_into().unwrap();
        crossover
    }

    fn random_tx() -> Transaction {
        // Create a transaction with randomised fields
        let mut tx = Transaction::default();

        let t = rand::thread_rng().gen_range(0, 8);
        tx.set_type(t.try_into().unwrap());

        tx.mut_payload()
            .set_anchor(BlsScalar::random(&mut rand::thread_rng()));

        let num_nuls = rand::thread_rng().gen_range(1, 4);
        for _ in 0..num_nuls {
            tx.mut_payload()
                .add_nullifier(BlsScalar::random(&mut rand::thread_rng()));
        }

        let num_notes = rand::thread_rng().gen_range(1, 2);
        for _ in 0..num_notes {
            tx.mut_payload().add_note(random_note());
        }

        tx.mut_payload().set_fee(random_fee());

        tx.mut_payload().set_crossover(random_crossover());

        let call_data_size = rand::thread_rng().gen_range(100, 1000);
        let call_data: Vec<u8> = (0..call_data_size)
            .map(|_| rand::thread_rng().gen::<u8>())
            .collect();

        tx.mut_payload()
            .set_proof(Proof::from_bytes(&DEFAULT_PROOF_BYTES).unwrap());

        tx.mut_payload().set_call_data(call_data);
        tx
    }

    #[test]
    fn transaction_encode_decode() {
        let tx = random_tx();
        let pbuf_tx: rusk_proto::Transaction = tx.clone().try_into().unwrap();
        let decoded_tx: Transaction = (&pbuf_tx).try_into().unwrap();

        assert_eq!(tx, decoded_tx);
    }

    #[test]
    fn transaction_read_write() {
        let mut tx = random_tx();

        let mut buf = [0u8; 4096];
        tx.read(&mut buf).unwrap();

        let mut decoded_tx = Transaction::default();
        decoded_tx.write(&mut buf).unwrap();

        assert_eq!(tx, decoded_tx);
    }
}
