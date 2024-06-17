// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubExtended, JubJubScalar};
use dusk_plonk::prelude::Proof;
use jubjub_schnorr::SignatureDouble;
use phoenix_core::transaction::Transaction;
use phoenix_core::{Crossover, Fee, Note, Ownable, SecretKey};
use poseidon_merkle::Opening as PoseidonOpening;
use rand_core::{CryptoRng, RngCore};
use rusk_abi::hash::Hasher;
use rusk_abi::{ContractId, CONTRACT_ID_BYTES, POSEIDON_TREE_DEPTH};

/// An input to a transaction that is yet to be proven.
#[derive(Debug, Clone)]
pub struct UnprovenTransactionInput {
    pub nullifier: BlsScalar,
    pub opening: PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>,
    pub note: Note,
    pub value: u64,
    pub blinder: JubJubScalar,
    pub npk_prime: JubJubExtended,
    pub sig: SignatureDouble,
}

impl UnprovenTransactionInput {
    pub fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        sk: &SecretKey,
        note: Note,
        value: u64,
        blinder: JubJubScalar,
        opening: PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>,
        tx_hash: BlsScalar,
    ) -> Self {
        let nullifier = note.gen_nullifier(sk);
        let nsk = sk.sk_r(note.stealth_address());
        let sig = nsk.sign_double(rng, tx_hash);

        let npk_prime = dusk_jubjub::GENERATOR_NUMS_EXTENDED * nsk.as_ref();

        Self {
            note,
            value,
            blinder,
            sig,
            nullifier,
            opening,
            npk_prime,
        }
    }

    /// Serialize the input to a variable size byte buffer.
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let affine_npk_p = JubJubAffine::from(&self.npk_prime);

        let opening_bytes = rkyv::to_bytes::<_, 256>(&self.opening)
            .expect("Rkyv serialization should always succeed for an opening")
            .to_vec();

        let mut bytes = Vec::with_capacity(
            BlsScalar::SIZE
                + Note::SIZE
                + JubJubAffine::SIZE
                + SignatureDouble::SIZE
                + u64::SIZE
                + JubJubScalar::SIZE
                + opening_bytes.len(),
        );

        bytes.extend_from_slice(&self.nullifier.to_bytes());
        bytes.extend_from_slice(&self.note.to_bytes());
        bytes.extend_from_slice(&self.value.to_bytes());
        bytes.extend_from_slice(&self.blinder.to_bytes());
        bytes.extend_from_slice(&affine_npk_p.to_bytes());
        bytes.extend_from_slice(&self.sig.to_bytes());
        bytes.extend(opening_bytes);

        bytes
    }

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

/// A transaction that is yet to be proven. The purpose of this is solely to
/// send to the node to perform a circuit proof.
#[derive(Debug, Clone)]
pub struct UnprovenTransaction {
    pub inputs: Vec<UnprovenTransactionInput>,
    pub outputs: Vec<(Note, u64, JubJubScalar)>,
    pub anchor: BlsScalar,
    pub fee: Fee,
    pub crossover: Option<(Crossover, u64, JubJubScalar)>,
    pub call: Option<(ContractId, String, Vec<u8>)>,
}

impl UnprovenTransaction {
    /// Consumes self and a proof to generate a transaction.
    pub fn prove(self, proof: Proof) -> Transaction {
        Transaction {
            anchor: self.anchor,
            nullifiers: self
                .inputs
                .into_iter()
                .map(|input| input.nullifier)
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .map(|(note, _, _)| note)
                .collect(),
            fee: self.fee,
            crossover: self.crossover.map(|c| c.0),
            proof: proof.to_bytes().to_vec(),
            call: self.call.map(|c| (c.0.to_bytes(), c.1, c.2)),
        }
    }

    /// Serialize the transaction to a variable length byte buffer.
    #[allow(unused_must_use)]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let serialized_inputs: Vec<Vec<u8>> = self
            .inputs
            .iter()
            .map(UnprovenTransactionInput::to_var_bytes)
            .collect();
        let num_inputs = self.inputs.len();
        let total_input_len = serialized_inputs
            .iter()
            .fold(0, |len, input| len + input.len());

        let serialized_outputs: Vec<
            [u8; Note::SIZE + u64::SIZE + JubJubScalar::SIZE],
        > = self
            .outputs
            .iter()
            .map(|(note, value, blinder)| {
                let mut buf = [0; Note::SIZE + u64::SIZE + JubJubScalar::SIZE];

                buf[..Note::SIZE].copy_from_slice(&note.to_bytes());
                buf[Note::SIZE..Note::SIZE + u64::SIZE]
                    .copy_from_slice(&value.to_bytes());
                buf[Note::SIZE + u64::SIZE
                    ..Note::SIZE + u64::SIZE + JubJubScalar::SIZE]
                    .copy_from_slice(&blinder.to_bytes());

                buf
            })
            .collect();
        let num_outputs = self.outputs.len();
        let total_output_len = serialized_outputs
            .iter()
            .fold(0, |len, output| len + output.len());

        let size = u64::SIZE
            + num_inputs * u64::SIZE
            + total_input_len
            + u64::SIZE
            + total_output_len
            + BlsScalar::SIZE
            + Fee::SIZE
            + u64::SIZE
            + self.crossover.map_or(0, |_| {
                Crossover::SIZE + u64::SIZE + JubJubScalar::SIZE
            })
            + u64::SIZE
            + self
                .call
                .as_ref()
                .map(|(_, cname, cdata)| {
                    CONTRACT_ID_BYTES + u64::SIZE + cname.len() + cdata.len()
                })
                .unwrap_or(0);

        let mut buf = vec![0; size];
        let mut writer = &mut buf[..];

        writer.write(&(num_inputs as u64).to_bytes());
        for sinput in serialized_inputs {
            writer.write(&(sinput.len() as u64).to_bytes());
            writer.write(&sinput);
        }

        writer.write(&(num_outputs as u64).to_bytes());
        for soutput in serialized_outputs {
            writer.write(&soutput);
        }

        writer.write(&self.anchor.to_bytes());
        writer.write(&self.fee.to_bytes());

        write_crossover_value_blinder(&mut writer, self.crossover);
        write_optional_call(&mut writer, &self.call);

        buf
    }

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

    /// Returns the anchor of the transaction.
    pub fn anchor(&self) -> BlsScalar {
        self.anchor
    }

    /// Returns the fee of the transaction.
    pub fn fee(&self) -> &Fee {
        &self.fee
    }

    /// Returns the crossover of the transaction.
    pub fn crossover(&self) -> Option<&(Crossover, u64, JubJubScalar)> {
        self.crossover.as_ref()
    }

    /// Returns the call of the transaction.
    pub fn call(&self) -> Option<&(ContractId, String, Vec<u8>)> {
        self.call.as_ref()
    }
}

/// Writes an optional call into the writer, prepending it with a `u64` denoting
/// if it is present or not. This should be called at the end of writing other
/// fields since it doesn't write any information about the length of the call
/// data.
fn write_optional_call<W: Write>(
    writer: &mut W,
    call: &Option<(ContractId, String, Vec<u8>)>,
) -> Result<(), BytesError> {
    match call {
        Some((cid, cname, cdata)) => {
            writer.write(&1_u64.to_bytes())?;

            writer.write(cid.as_bytes())?;

            let cname_len = cname.len() as u64;
            writer.write(&cname_len.to_bytes())?;
            writer.write(cname.as_bytes())?;

            writer.write(cdata)?;
        }
        None => {
            writer.write(&0_u64.to_bytes())?;
        }
    };

    Ok(())
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

fn write_crossover_value_blinder<W: Write>(
    writer: &mut W,
    crossover: Option<(Crossover, u64, JubJubScalar)>,
) -> Result<(), BytesError> {
    match crossover {
        Some((crossover, value, blinder)) => {
            writer.write(&1_u64.to_bytes())?;
            writer.write(&crossover.to_bytes())?;
            writer.write(&value.to_bytes())?;
            writer.write(&blinder.to_bytes())?;
        }
        None => {
            writer.write(&0_u64.to_bytes())?;
        }
    }

    Ok(())
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
