// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::NodeClient;

use alloc::vec::Vec;
use core::mem;

use canonical::{Canon, Sink, Source};
use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubExtended};
use dusk_pki::{Ownable, SecretSpendKey};
use dusk_plonk::prelude::{JubJubScalar, Proof};
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Proof as SchnorrSig;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};
use rusk_abi::hash::Hasher;
use rusk_abi::{ContractId, POSEIDON_TREE_DEPTH};

const CONTRACT_ID_SIZE: usize = mem::size_of::<ContractId>();

/// The structure sent over the network representing a transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    nullifiers: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    proof: Proof,
    fee: Fee,
    crossover: Crossover,
    call: Option<(ContractId, Vec<u8>)>,
}

impl Transaction {
    /// Creates a transaction from the skeleton and the proof.
    fn new(tx_skel: TransactionSkeleton, proof: Proof) -> Self {
        Self {
            proof,
            nullifiers: tx_skel.nullifiers,
            outputs: tx_skel.outputs,
            anchor: tx_skel.anchor,
            fee: tx_skel.fee,
            crossover: tx_skel.crossover,
            call: tx_skel.call,
        }
    }

    /// Hashes the transaction excluding.
    pub fn hash(&self) -> BlsScalar {
        let skel = TransactionSkeleton::from(self.clone());
        skel.hash()
    }

    /// Serializes the transaction into a variable length byte buffer.
    pub fn to_bytes(&self) -> Result<Vec<u8>, BytesError> {
        // compute the serialized size to preallocate space
        let size = u64::SIZE
            + self.nullifiers.len() * BlsScalar::SIZE
            + u64::SIZE
            + self.outputs.len() * Note::SIZE
            + BlsScalar::SIZE
            + Fee::SIZE
            + Proof::SIZE
            + Crossover::SIZE
            + u64::SIZE
            + self
                .call
                .as_ref()
                .map(|(_, cdata)| CONTRACT_ID_SIZE + cdata.len())
                .unwrap_or(0);

        let mut bytes = vec![0u8; size];
        let mut writer = &mut bytes[..];

        writer.write(&(self.nullifiers.len() as u64).to_bytes())?;
        for input in &self.nullifiers {
            writer.write(&input.to_bytes())?;
        }

        writer.write(&(self.outputs.len() as u64).to_bytes())?;
        for output in &self.outputs {
            writer.write(&output.to_bytes())?;
        }

        writer.write(&self.anchor.to_bytes())?;
        writer.write(&self.fee.to_bytes())?;
        writer.write(&self.proof.to_bytes())?;
        writer.write(&self.crossover.to_bytes())?;

        write_optional_call(&mut writer, self.call.as_ref())?;

        Ok(bytes)
    }

    /// Deserializes the transaction from a bytes buffer.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buffer = buf;

        let num_inputs = u64::from_reader(&mut buffer)? as usize;
        let mut nullifiers = Vec::with_capacity(num_inputs);

        for _ in 0..num_inputs {
            nullifiers.push(BlsScalar::from_reader(&mut buffer)?);
        }

        let num_outputs = u64::from_reader(&mut buffer)? as usize;
        let mut outputs = Vec::with_capacity(num_outputs);

        for _ in 0..num_outputs {
            outputs.push(Note::from_reader(&mut buffer)?);
        }

        let anchor = BlsScalar::from_reader(&mut buffer)?;
        let fee = Fee::from_reader(&mut buffer)?;
        let proof = Proof::from_reader(&mut buffer)?;
        let crossover = Crossover::from_reader(&mut buffer)?;

        let call = read_optional_call(&mut buffer)?;

        Ok(Self {
            nullifiers,
            outputs,
            anchor,
            fee,
            crossover,
            call,
            proof,
        })
    }

    /// The nullifiers in the transaction.
    pub fn inputs(&self) -> &[BlsScalar] {
        &self.nullifiers
    }

    /// The output notes of the transaction.
    pub fn outputs(&self) -> &[Note] {
        &self.outputs
    }

    /// The anchor of the transaction.
    pub fn anchor(&self) -> BlsScalar {
        self.anchor
    }

    /// The proof of thes transaction.
    pub fn proof(&self) -> &Proof {
        &self.proof
    }

    /// The fee of the transaction.
    pub fn fee(&self) -> &Fee {
        &self.fee
    }

    /// The crossover of the transaction.
    pub fn crossover(&self) -> &Crossover {
        &self.crossover
    }

    /// The call data of the transaction.
    pub fn call(&self) -> Option<&(ContractId, Vec<u8>)> {
        self.call.as_ref()
    }
}

/// Transaction skeleton.
struct TransactionSkeleton {
    nullifiers: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: Crossover,
    call: Option<(ContractId, Vec<u8>)>,
}

impl TransactionSkeleton {
    fn new(
        nullifiers: Vec<BlsScalar>,
        outputs: Vec<Note>,
        anchor: BlsScalar,
        fee: Fee,
        crossover: Crossover,
        call: Option<(ContractId, Vec<u8>)>,
    ) -> Self {
        Self {
            nullifiers,
            outputs,
            anchor,
            fee,
            crossover,
            call,
        }
    }

    fn hash(&self) -> BlsScalar {
        let mut hasher = Hasher::new();

        for nullifier in &self.nullifiers {
            hasher.update(nullifier.to_bytes());
        }
        for note in &self.outputs {
            hasher.update(note.to_bytes());
        }

        hasher = hasher
            .chain_update(self.anchor.to_bytes())
            .chain_update(self.fee.to_bytes())
            .chain_update(self.crossover.to_bytes());

        if let Some((cid, cdata)) = &self.call {
            hasher.update(cid.as_bytes());
            hasher.update(cdata);
        }

        hasher.finalize()
    }
}

impl From<Transaction> for TransactionSkeleton {
    fn from(tx: Transaction) -> Self {
        Self {
            nullifiers: tx.nullifiers,
            outputs: tx.outputs,
            anchor: tx.anchor,
            fee: tx.fee,
            crossover: tx.crossover,
            call: tx.call,
        }
    }
}

impl From<UnprovenTransaction> for TransactionSkeleton {
    fn from(utx: UnprovenTransaction) -> Self {
        Self {
            nullifiers: utx
                .inputs
                .iter()
                .map(UnprovenTransactionInput::nullifier)
                .collect(),
            outputs: utx.outputs.iter().map(|o| o.0).collect(),
            anchor: utx.anchor,
            fee: utx.fee,
            crossover: utx.crossover.0,
            call: utx.call,
        }
    }
}

/// An input to a transaction that is yet to be proven.
#[derive(Debug, Clone)]
pub struct UnprovenTransactionInput {
    nullifier: BlsScalar,
    opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
    note: Note,
    value: u64,
    blinder: JubJubScalar,
    pk_r_prime: JubJubExtended,
    sig: SchnorrSig,
}

impl UnprovenTransactionInput {
    fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        ssk: &SecretSpendKey,
        note: Note,
        value: u64,
        blinder: JubJubScalar,
        opening: PoseidonBranch<POSEIDON_TREE_DEPTH>,
        tx_hash: BlsScalar,
    ) -> Self {
        let nullifier = note.gen_nullifier(ssk);
        let sk_r = ssk.sk_r(note.stealth_address());
        let sig = SchnorrSig::new(&sk_r, rng, tx_hash);

        let pk_r_prime = dusk_jubjub::GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

        Self {
            note,
            value,
            blinder,
            sig,
            nullifier,
            opening,
            pk_r_prime,
        }
    }

    /// Serialize the input to a variable size byte buffer.
    pub fn to_bytes(&self) -> Vec<u8> {
        let affine_pkr = JubJubAffine::from(&self.pk_r_prime);

        // TODO Magic number for the buffer size here.
        // Should be corrected once dusk-poseidon implements `Serializable` for
        // `PoseidonBranch`.
        let mut opening_bytes = [0; opening_buf_size(POSEIDON_TREE_DEPTH)];
        let mut sink = Sink::new(&mut opening_bytes[..]);
        self.opening.encode(&mut sink);

        let mut bytes = Vec::with_capacity(
            BlsScalar::SIZE
                + Note::SIZE
                + JubJubAffine::SIZE
                + SchnorrSig::SIZE
                + u64::SIZE
                + JubJubScalar::SIZE
                + opening_bytes.len(),
        );

        bytes.extend_from_slice(&self.nullifier.to_bytes());
        bytes.extend_from_slice(&self.note.to_bytes());
        bytes.extend_from_slice(&self.value.to_bytes());
        bytes.extend_from_slice(&self.blinder.to_bytes());
        bytes.extend_from_slice(&affine_pkr.to_bytes());
        bytes.extend_from_slice(&self.sig.to_bytes());
        bytes.extend_from_slice(&opening_bytes);

        bytes
    }

    /// Deserializes the the input from bytes.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut bytes = buf;

        let nullifier = BlsScalar::from_reader(&mut bytes)?;
        let note = Note::from_reader(&mut bytes)?;
        let value = u64::from_reader(&mut bytes)?;
        let blinder = JubJubScalar::from_reader(&mut bytes)?;
        let pk_r_prime =
            JubJubExtended::from(JubJubAffine::from_reader(&mut bytes)?);
        let sig = SchnorrSig::from_reader(&mut bytes)?;

        let mut source = Source::new(bytes);
        let opening = PoseidonBranch::decode(&mut source)
            .map_err(|_| BytesError::InvalidData)?;

        Ok(Self {
            note,
            value,
            blinder,
            sig,
            nullifier,
            opening,
            pk_r_prime,
        })
    }

    /// Returns the nullifier of the input.
    pub fn nullifier(&self) -> BlsScalar {
        self.nullifier
    }

    /// Returns the opening of the input.
    pub fn opening(&self) -> &PoseidonBranch<POSEIDON_TREE_DEPTH> {
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

    /// Returns the input's pk_r'.
    pub fn pk_r_prime(&self) -> JubJubExtended {
        self.pk_r_prime
    }

    /// Returns the input's signature.
    pub fn signature(&self) -> &SchnorrSig {
        &self.sig
    }
}

const fn opening_buf_size(depth: usize) -> usize {
    (depth + 2) * (BlsScalar::SIZE * 5 + 8)
}

/// A transaction that is yet to be proven. The purpose of this is solely to
/// send to the node to perform a circuit proof.
#[derive(Debug, Clone)]
pub struct UnprovenTransaction {
    inputs: Vec<UnprovenTransactionInput>,
    outputs: Vec<(Note, u64, JubJubScalar)>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: (Crossover, u64, JubJubScalar),
    call: Option<(ContractId, Vec<u8>)>,
}

impl UnprovenTransaction {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new<Rng: RngCore + CryptoRng, C: NodeClient>(
        rng: &mut Rng,
        node: &C,
        sender: &SecretSpendKey,
        inputs: Vec<(Note, u64, JubJubScalar)>,
        outputs: Vec<(Note, u64, JubJubScalar)>,
        anchor: BlsScalar,
        fee: Fee,
        crossover: (Crossover, u64, JubJubScalar),
        call: Option<(ContractId, Vec<u8>)>,
    ) -> Result<Self, C::Error> {
        let nullifiers: Vec<BlsScalar> = inputs
            .iter()
            .map(|(note, _, _)| note.gen_nullifier(sender))
            .collect();

        let mut openings = Vec::with_capacity(inputs.len());
        for (note, _, _) in &inputs {
            let opening = node.fetch_opening(note)?;
            openings.push(opening);
        }

        let skel = TransactionSkeleton::new(
            nullifiers,
            outputs.iter().map(|o| o.0).collect(),
            anchor,
            fee,
            crossover.0,
            call,
        );
        let hash = skel.hash();

        let inputs: Vec<UnprovenTransactionInput> = inputs
            .into_iter()
            .zip(openings.into_iter())
            .map(|((note, value, blinder), opening)| {
                UnprovenTransactionInput::new(
                    rng, sender, note, value, blinder, opening, hash,
                )
            })
            .collect();

        Ok(Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call: skel.call,
        })
    }

    /// Consumes self and a proof to generate a transaction.
    pub(crate) fn prove(self, proof: Proof) -> Transaction {
        let skel = TransactionSkeleton::from(self);
        Transaction::new(skel, proof)
    }

    /// Serialize the transaction to a variable length byte buffer.
    pub fn to_bytes(&self) -> Result<Vec<u8>, BytesError> {
        let serialized_inputs: Vec<Vec<u8>> = self
            .inputs
            .iter()
            .map(UnprovenTransactionInput::to_bytes)
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
            + Crossover::SIZE
            + u64::SIZE
            + JubJubScalar::SIZE
            + u64::SIZE
            + self
                .call
                .as_ref()
                .map(|(_, cdata)| CONTRACT_ID_SIZE + cdata.len())
                .unwrap_or(0);

        let mut buf = vec![0; size];
        let mut writer = &mut buf[..];

        writer.write(&(num_inputs as u64).to_bytes())?;
        for sinput in serialized_inputs {
            writer.write(&(sinput.len() as u64).to_bytes())?;
            writer.write(&sinput)?;
        }

        writer.write(&(num_outputs as u64).to_bytes())?;
        for soutput in serialized_outputs {
            writer.write(&soutput)?;
        }

        writer.write(&self.anchor.to_bytes())?;
        writer.write(&self.fee.to_bytes())?;

        writer.write(&self.crossover.0.to_bytes())?;
        writer.write(&self.crossover.1.to_bytes())?;
        writer.write(&self.crossover.2.to_bytes())?;

        write_optional_call(&mut writer, self.call.as_ref())?;

        Ok(buf)
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

        let c = Crossover::from_reader(&mut buffer)?;
        let value = u64::from_reader(&mut buffer)?;
        let blinder = JubJubScalar::from_reader(&mut buffer)?;

        let crossover = (c, value, blinder);

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
        TransactionSkeleton::from(self.clone()).hash()
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
    pub fn crossover(&self) -> &(Crossover, u64, JubJubScalar) {
        &self.crossover
    }

    /// Returns the call of the transaction.
    pub fn call(&self) -> Option<&(ContractId, Vec<u8>)> {
        self.call.as_ref()
    }
}

/// Writes an optional call into the writer, prepending it with a `u64` denoting
/// if it is present or not. This should be called at the end of writing other
/// fields since it doesn't write any information about the length of the call
/// data.
fn write_optional_call<W: Write>(
    writer: &mut W,
    call: Option<&(ContractId, Vec<u8>)>,
) -> Result<(), BytesError> {
    match call {
        Some((cid, cdata)) => {
            writer.write(&1_u64.to_bytes())?;
            writer.write(cid.as_bytes())?;
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
) -> Result<Option<(ContractId, Vec<u8>)>, BytesError> {
    let mut call = None;

    if u64::from_reader(buffer)? != 0 {
        let buf_len = buffer.len();

        // needs to be at least the size of a contract ID and have some call
        // data.
        if buf_len < CONTRACT_ID_SIZE {
            return Err(BytesError::BadLength {
                found: buf_len,
                expected: CONTRACT_ID_SIZE,
            });
        }
        let (cid_buffer, cdata_buffer) = buffer.split_at(CONTRACT_ID_SIZE);

        let contract_id = ContractId::from(cid_buffer);
        let call_data = Vec::from(cdata_buffer);

        call = Some((contract_id, call_data));
    }

    Ok(call)
}
