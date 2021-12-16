// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::POSEIDON_BRANCH_DEPTH;

use alloc::vec::Vec;
use core::mem;

use dusk_abi::ContractId;
use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::{BlsScalar, JubJubExtended};
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
    crossover: Option<Crossover>,
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
            + u64::SIZE
            + self.crossover.map(|_| Crossover::SIZE).unwrap_or(0)
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

        let mut crossover = None;
        if u64::from_reader(&mut buffer)? != 0 {
            crossover = Some(Crossover::from_reader(&mut buffer)?);
        }

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
    crossover: Option<Crossover>,
    call: Option<(ContractId, Vec<u8>)>,
}

impl TransactionSkeleton {
    pub(crate) fn new(
        inputs: Vec<BlsScalar>,
        outputs: Vec<Note>,
        anchor: BlsScalar,
        fee: Fee,
        crossover: Option<Crossover>,
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

impl From<UnprovenTransaction> for TransactionSkeleton {
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
            crossover: utx.crossover.map(|c| c.0),
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

pub(crate) struct UnprovenTransactionInput {
    sig: SchnorrSig,
    nullifier: BlsScalar,
    note: Note,
    // FIXME magic number
    opening: PoseidonBranch<POSEIDON_BRANCH_DEPTH>,
    pk_rprime: JubJubExtended,
}

impl UnprovenTransactionInput {
    pub fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        ssk: &SecretSpendKey,
        note: Note,
        opening: PoseidonBranch<POSEIDON_BRANCH_DEPTH>,
        tx_hash: BlsScalar,
    ) -> Self {
        let nullifier = note.gen_nullifier(ssk);
        let sk_r = ssk.sk_r(note.stealth_address());
        let sig = SchnorrSig::new(&sk_r, rng, tx_hash);

        let pk_rprime = dusk_jubjub::GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

        Self {
            sig,
            nullifier,
            note,
            opening,
            pk_rprime,
        }
    }

    fn nullifier(&self) -> BlsScalar {
        self.nullifier
    }
}

/// A transaction that is yet to be proven.
pub(crate) struct UnprovenTransaction {
    inputs: Vec<UnprovenTransactionInput>,
    outputs: Vec<(Note, u64, JubJubScalar)>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: Option<(Crossover, u64, JubJubScalar)>,
    call: Option<(ContractId, Vec<u8>)>,
}

impl UnprovenTransaction {
    pub(crate) fn delegate_prove(self) -> Transaction {
        // Keep in mind that this needs access to the prover key which is a
        // couple of gigs. This will probably have to be a network component.
        todo!()
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
