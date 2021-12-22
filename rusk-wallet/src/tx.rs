// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use core::mem;

use canonical::{Canon, Sink, Source};
use dusk_abi::ContractId;
use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubExtended};
use dusk_pki::{Ownable, SecretSpendKey};
use dusk_plonk::prelude::{JubJubScalar, Proof};
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge::hash;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Proof as SchnorrSig;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

const CONTRACT_ID_SIZE: usize = mem::size_of::<ContractId>();

/// The structure sent over the network representing a transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    inputs: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    proof: Proof,
    fee: Fee,
    crossover: Crossover,
    call: Option<(ContractId, Vec<u8>)>,
}

impl Transaction {
    /// Creates a transaction from the skeleten and the proof.
    pub(crate) fn new(tx_skel: TransactionSkeleton, proof: Proof) -> Self {
        Self {
            proof,
            inputs: tx_skel.inputs,
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

    /// Returns the inputs to the hash function.
    pub fn hash_inputs(&self) -> Vec<BlsScalar> {
        let skel = TransactionSkeleton::from(self.clone());
        skel.hash_inputs()
    }

    /// Serializes the transaction into a variable length byte buffer.
    pub fn to_var_bytes(&self) -> Result<Vec<u8>, BytesError> {
        // compute the serialized size to preallocate space
        let size = u64::SIZE
            + self.inputs.len() * BlsScalar::SIZE
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

        writer.write(&(self.inputs.len() as u64).to_bytes())?;
        for input in &self.inputs {
            writer.write(&input.to_bytes())?;
        }

        writer.write(&(self.outputs.len() as u64).to_bytes())?;
        for output in &self.outputs {
            writer.write(&output.to_bytes())?;
        }

        writer.write(&self.anchor.to_bytes())?;
        writer.write(&self.fee.to_bytes())?;
        writer.write(&self.proof.to_bytes())?;

        match &self.crossover {
            None => {
                writer.write(&0_u64.to_bytes())?;
            }
            Some(c) => {
                writer.write(&1_u64.to_bytes())?;
                writer.write(&c.to_bytes())?;
            }
        }

        match &self.call {
            None => {
                writer.write(&0_u64.to_bytes())?;
            }
            Some((cid, cdata)) => {
                writer.write(&1_u64.to_bytes())?;
                writer.write(cid.as_bytes())?;
                writer.write(cdata)?;
            }
        }

        Ok(bytes)
    }

    /// Deserializes the transaction from a bytes buffer.
    pub fn from_bytes<B: AsRef<[u8]>>(buf: B) -> Result<Self, BytesError> {
        let mut buffer = buf.as_ref();

        let ninputs = u64::from_reader(&mut buffer)? as usize;
        let mut inputs = Vec::with_capacity(ninputs);

        for _ in 0..ninputs {
            inputs.push(BlsScalar::from_reader(&mut buffer)?);
        }

        let noutputs = u64::from_reader(&mut buffer)? as usize;
        let mut outputs = Vec::with_capacity(noutputs);

        for _ in 0..noutputs {
            outputs.push(Note::from_reader(&mut buffer)?);
        }

        let anchor = BlsScalar::from_reader(&mut buffer)?;
        let fee = Fee::from_reader(&mut buffer)?;
        let proof = Proof::from_reader(&mut buffer)?;

        let crossover = Crossover::from_reader(&mut buffer)?;

        let mut call = None;
        if u64::from_reader(&mut buffer)? != 0 {
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

        Ok(Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
            proof,
        })
    }
}

/// Transaction skeleton.
pub(crate) struct TransactionSkeleton {
    inputs: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: Crossover,
    call: Option<(ContractId, Vec<u8>)>,
}

impl TransactionSkeleton {
    pub(crate) fn new(
        inputs: Vec<BlsScalar>,
        outputs: Vec<Note>,
        anchor: BlsScalar,
        fee: Fee,
        crossover: Crossover,
        call: Option<(ContractId, Vec<u8>)>,
    ) -> Self {
        Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
        }
    }
}

impl From<Transaction> for TransactionSkeleton {
    fn from(tx: Transaction) -> Self {
        Self {
            inputs: tx.inputs,
            outputs: tx.outputs,
            anchor: tx.anchor,
            fee: tx.fee,
            crossover: tx.crossover,
            call: tx.call,
        }
    }
}

impl From<UnprovenTransaction>
    for TransactionSkeleton
{
    fn from(utx: UnprovenTransaction) -> Self {
        Self {
            inputs: utx
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

impl TransactionSkeleton {
    pub(crate) fn hash(&self) -> BlsScalar {
        hash(&self.hash_inputs())
    }

    pub(crate) fn hash_inputs(&self) -> Vec<BlsScalar> {
        let size = self.inputs.len()
            + 12 * self.outputs.len()
            + 1
            + 4
            + self
            .crossover
            .map(|_| 3 + PoseidonCipher::cipher_size())
            .unwrap_or(0)
            // When this lands the weird logic checking if there needs to be
            // padding is gone. https://github.com/rust-lang/rust/issues/88581
            + self
            .call.as_ref()
            .map(|(_, cdata)| {
                8 + cdata.len() / (2*BlsScalar::SIZE)
                    + if cdata.len() % (2*BlsScalar::SIZE) == 0 { 0 } else { 1 }
            })
            .unwrap_or(0);

        let mut hash_inputs = Vec::with_capacity(size);

        hash_inputs.append(&mut self.inputs.clone());
        self.outputs.iter().for_each(|note| {
            hash_inputs.append(&mut note.hash_inputs().to_vec());
        });
        hash_inputs.push(self.anchor);
        hash_inputs.append(&mut fee_hash_inputs(&self.fee).to_vec());

        if let Some(c) = &self.crossover {
            hash_inputs.append(&mut c.to_hash_inputs().to_vec())
        }

        if let Some((cid, cdata)) = &self.call {
            hash_inputs.append(&mut hash_inputs_from_bytes(cid.as_bytes()));
            hash_inputs.append(&mut hash_inputs_from_bytes(cdata));
        }

        hash_inputs
    }
}

/// An input to a transaction that is yet to be proven.
#[derive(Debug, Clone)]
pub struct UnprovenTransactionInput {
    nullifier: BlsScalar,
    opening: PoseidonBranch<POSEIDON_DEPTH>,
    note: Note,
    pk_r_prime: JubJubExtended,
    value: u64,
    blinder: JubJubScalar,
    sig: SchnorrSig,
}

impl UnprovenTransactionInput {
    pub fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        ssk: &SecretSpendKey,
        note: Note,
        value: u64,
        blinder: JubJubScalar,
        opening: PoseidonBranch<POSEIDON_DEPTH>,
        tx_hash: BlsScalar,
    ) -> Self {
        let nullifier = note.gen_nullifier(ssk);
        let sk_r = ssk.sk_r(note.stealth_address());
        let sig = SchnorrSig::new(&sk_r, rng, tx_hash);

        let pk_r_prime = dusk_jubjub::GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

        Self {
            sig,
            nullifier,
            note,
            opening,
            pk_r_prime,
            value,
            blinder,
        }
    }

    /// Serialize the input to a variable size byte buffer.
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let affine_pkr = JubJubAffine::from(&self.pk_r_prime);

        // TODO Magic number for the buffer size here.
        // Should be corrected once dusk-poseidon implements `Serializable` for
        // `PoseidonBranch`.
        let mut opening_bytes = [0; opening_buf_size(POSEIDON_DEPTH)];
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
    pub fn from_bytes(buf: &[u8]) -> Result<Self, BytesError> {
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
            nullifier,
            note,
            value,
            blinder,
            pk_r_prime,
            sig,
            opening,
        })
    }

    /// Returns the nullifier of the input.
    pub fn nullifier(&self) -> BlsScalar {
        self.nullifier
    }

    /// Returns the opening of the input.
    pub fn opening(&self) -> &PoseidonBranch<POSEIDON_DEPTH> {
        &self.opening
    }

    /// Returns the note of the input.
    pub fn note(&self) -> &Note {
        &self.note
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
    pub fn new(
        inputs: Vec<UnprovenTransactionInput>,
        outputs: Vec<(Note, u64, JubJubScalar)>,
        anchor: BlsScalar,
        fee: Fee,
        crossover: (Crossover, u64, JubJubScalar),
        call: Option<(ContractId, Vec<u8>)>,
    ) -> Self {
        Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
        }
    }

    pub fn delegate_prove(&self) -> Transaction {
        // Keep in mind that this needs access to the prover key which is a
        // couple of gigs. This will probably have to be a network component.
        todo!()
    }

    /// Proves the given unproven transaction.
    pub fn prove(&self) -> Transaction {
        todo!()
    }

    /// Serialize the transaction to a variable length byte buffer.
    pub fn to_var_bytes(&self) -> Result<Vec<u8>, BytesError> {
        let serialized_inputs: Vec<Vec<u8>> = self
            .inputs
            .iter()
            .map(UnprovenTransactionInput::to_var_bytes)
            .collect();
        let ninputs = self.inputs.len();
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
        let noutputs = self.outputs.len();
        let total_output_len = serialized_outputs
            .iter()
            .fold(0, |len, output| len + output.len());

        let size = u64::SIZE
            + ninputs * u64::SIZE
            + total_input_len
            + u64::SIZE
            + total_output_len
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

        writer.write(&(ninputs as u64).to_bytes())?;
        for sinput in serialized_inputs {
            writer.write(&(sinput.len() as u64).to_bytes())?;
            writer.write(&sinput)?;
        }

        writer.write(&(noutputs as u64).to_bytes())?;
        for soutput in serialized_outputs {
            writer.write(&soutput)?;
        }

        writer.write(&self.anchor.to_bytes())?;
        writer.write(&self.fee.to_bytes())?;

        writer.write(&self.crossover.0.to_bytes())?;
        writer.write(&self.crossover.1.to_bytes())?;
        writer.write(&self.crossover.2.to_bytes())?;

        match &self.call {
            None => {
                writer.write(&0_u64.to_bytes())?;
            }
            Some((cid, cdata)) => {
                writer.write(&1_u64.to_bytes())?;
                writer.write(cid.as_bytes())?;
                writer.write(cdata)?;
            }
        }

        Ok(buf)
    }

    /// Deserialize the transaction from a bytes buffer.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buffer = buf;

        let ninputs = u64::from_reader(&mut buffer)?;
        let mut inputs = Vec::with_capacity(ninputs as usize);
        for _ in 0..ninputs {
            let size = u64::from_reader(&mut buffer)?;
            inputs.push(UnprovenTransactionInput::from_bytes(
                &buffer[..size as usize],
            )?);
            buffer = &buffer[..size as usize];
        }

        let noutputs = u64::from_reader(&mut buffer)?;
        let mut outputs = Vec::with_capacity(noutputs as usize);
        for _ in 0..noutputs {
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

        let mut call = None;
        if u64::from_reader(&mut buffer)? != 0 {
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

    /// Returns the inputs to a hash function.
    pub fn hash_inputs(&self) -> Vec<BlsScalar> {
        TransactionSkeleton::from(self.clone()).hash_inputs()
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

/// Returns hash inputs from a slice of bytes, padding to zero.
fn hash_inputs_from_bytes<B: AsRef<[u8]>>(bytes: B) -> Vec<BlsScalar> {
    bytes
        .as_ref()
        .chunks(2 * BlsScalar::SIZE)
        .map(|c| {
            let mut wide = [0u8; 64];
            (&mut wide[..c.len()]).copy_from_slice(c);
            BlsScalar::from_bytes_wide(&wide)
        })
        .collect()
}

// Will become redundant when this lands
// https://github.com/dusk-network/phoenix-core/issues/100
fn fee_hash_inputs(fee: &Fee) -> [BlsScalar; 4] {
    let pk_r = fee.stealth_address().pk_r().as_ref().to_hash_inputs();

    [
        BlsScalar::from(fee.gas_limit),
        BlsScalar::from(fee.gas_price),
        pk_r[0],
        pk_r[1],
    ]
}
