// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by the wallets and the circuit to represent transations
//! before proving. This will be used by the wallets and other users
//! who wish to sign transations offline

use alloc::string::String;
use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Error as BytesError};
use jubjub_schnorr::SignatureDouble;
use piecrust_uplink::{ContractId, CONTRACT_ID_BYTES};
use poseidon_merkle::Opening as PoseidonOpening;

use crate::hash::Hasher;

use super::*;

/// Constant depth of the merkle tree that provides the opening proofs.
pub const POSEIDON_TREE_DEPTH: usize = 17;

/// An input to a transaction that is yet to be proven.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct UnprovenTransactionInput {
    nullifier: BlsScalar,
    opening: PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>,
    note: Note,
    value: u64,
    blinder: JubJubScalar,
    npk_prime: JubJubExtended,
    sig: SignatureDouble,
}

/// A transaction that is yet to be proven. The purpose of this is solely to
/// send to the node to perform a circuit proof.
/// The fields are made public to avoid clippy warnings and constructors which
/// require rng and hashing crates
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct UnprovenTransaction {
    inputs: Vec<UnprovenTransactionInput>,
    outputs: Vec<(Note, u64, JubJubScalar)>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: Option<(Crossover, u64, JubJubScalar)>,
    call: Option<(ContractId, String, Vec<u8>)>,
}

impl UnprovenTransaction {
    /// Deserialize the transaction from a bytes buffer.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buffer = buf;

        let num_inputs = u64::from_reader(&mut buffer)?;
        let mut inputs = Vec::with_capacity(num_inputs as usize);
        for _ in 0..num_inputs {
            let size = u64::from_reader(&mut buffer)? as usize;
            inputs.push(UnprovenTransactionInput::from_slice(&buffer[..size])?);
            buffer = &buffer[size..];
        }

        let num_outputs = u64::from_reader(&mut buffer)?;
        let mut outputs = Vec::with_capacity(num_outputs as usize);
        for _ in 0..num_outputs {
            let note = Note::from_reader(&mut buffer)?;
            let value = u64::from_reader(&mut buffer)?;
            let blinder = JubJubScalar::from_reader(&mut buffer)?;

            outputs.push((note, value, blinder));
        }

        let anchor = BlsScalar::from_reader(&mut buffer)?;
        let fee = Fee::from_reader(&mut buffer)?;

        let crossover = read_crossover_value_blinder(&mut buffer)?;

        let call = read_optional_call(&mut buffer)?;

        Ok(Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
        })
    }
    /// Returns the hash of the transaction.
    pub fn hash(&self) -> BlsScalar {
        let nullifiers: Vec<BlsScalar> =
            self.inputs.iter().map(|input| input.nullifier).collect();

        let hash_outputs: Vec<Note> =
            self.outputs.iter().map(|(note, _, _)| *note).collect();
        let hash_crossover = self.crossover.map(|c| c.0);
        let hash_bytes = self.call.clone().map(|c| (c.0.to_bytes(), c.1, c.2));

        Hasher::digest(Transaction::hash_input_bytes_from_components(
            &nullifiers,
            &hash_outputs,
            &self.anchor,
            &self.fee,
            &hash_crossover,
            &hash_bytes,
        ))
    }

    /// Returns the inputs to the transaction.
    pub fn inputs(&self) -> &[UnprovenTransactionInput] {
        &self.inputs
    }

    /// Returns the outputs of the transaction.
    pub fn outputs(&self) -> &[(Note, u64, JubJubScalar)] {
        &self.outputs
    }

    /// Returns the crossover of the transaction.
    pub fn crossover(&self) -> Option<&(Crossover, u64, JubJubScalar)> {
        self.crossover.as_ref()
    }

    /// Returns the fee of the transaction.
    pub fn fee(&self) -> &Fee {
        &self.fee
    }
}

impl UnprovenTransactionInput {
    /// Deserializes the the input from bytes.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut bytes = buf;

        let nullifier = BlsScalar::from_reader(&mut bytes)?;
        let note = Note::from_reader(&mut bytes)?;
        let value = u64::from_reader(&mut bytes)?;
        let blinder = JubJubScalar::from_reader(&mut bytes)?;
        let npk_prime =
            JubJubExtended::from(JubJubAffine::from_reader(&mut bytes)?);
        let sig = SignatureDouble::from_reader(&mut bytes)?;

        // `to_vec` is required here otherwise `rkyv` will throw an alignment
        // error
        #[allow(clippy::unnecessary_to_owned)]
        let opening = rkyv::from_bytes(&bytes.to_vec())
            .map_err(|_| BytesError::InvalidData)?;

        Ok(Self {
            note,
            value,
            blinder,
            sig,
            nullifier,
            opening,
            npk_prime,
        })
    }

    /// Returns the nullifier of the input.
    pub fn nullifier(&self) -> BlsScalar {
        self.nullifier
    }

    /// Returns the opening of the input.
    pub fn opening(&self) -> &PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4> {
        &self.opening
    }

    /// Returns the note of the input.
    pub fn note(&self) -> &Note {
        &self.note
    }

    /// Returns the value of the input.
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Returns the blinding factor of the input.
    pub fn blinding_factor(&self) -> JubJubScalar {
        self.blinder
    }

    /// Returns the input's note public key prime.
    pub fn note_pk_prime(&self) -> JubJubExtended {
        self.npk_prime
    }

    /// Returns the input's signature.
    pub fn signature(&self) -> &SignatureDouble {
        &self.sig
    }
}

/// Reads an optional crossover from the given buffer.
fn read_crossover_value_blinder(
    buffer: &mut &[u8],
) -> Result<Option<(Crossover, u64, JubJubScalar)>, BytesError> {
    let ser = match u64::from_reader(buffer)? {
        0 => None,
        _ => {
            let crossover = Crossover::from_reader(buffer)?;
            let value = u64::from_reader(buffer)?;
            let blinder = JubJubScalar::from_reader(buffer)?;
            Some((crossover, value, blinder))
        }
    };

    Ok(ser)
}

/// Reads an optional call from the given buffer. This should be called at the
/// end of parsing other fields since it consumes the entirety of the buffer.
fn read_optional_call(
    buffer: &mut &[u8],
) -> Result<Option<(ContractId, String, Vec<u8>)>, BytesError> {
    let mut call = None;

    if u64::from_reader(buffer)? != 0 {
        let buf_len = buffer.len();

        // needs to be at least the size of a contract ID and have some call
        // data.
        if buf_len < CONTRACT_ID_BYTES {
            return Err(BytesError::BadLength {
                found: buf_len,
                expected: CONTRACT_ID_BYTES,
            });
        }
        let (mid_buffer, mut buffer_left) = {
            let (buf, left) = buffer.split_at(CONTRACT_ID_BYTES);

            let mut mid_buf = [0u8; CONTRACT_ID_BYTES];
            mid_buf.copy_from_slice(buf);

            (mid_buf, left)
        };

        let contract_id = ContractId::from(mid_buffer);

        let buffer = &mut buffer_left;

        let cname_len = u64::from_reader(buffer)?;
        let (cname_bytes, buffer_left) = buffer.split_at(cname_len as usize);

        let cname = String::from_utf8(cname_bytes.to_vec())
            .map_err(|_| BytesError::InvalidData)?;

        let call_data = Vec::from(buffer_left);
        call = Some((contract_id, cname, call_data));
    }

    Ok(call)
}
