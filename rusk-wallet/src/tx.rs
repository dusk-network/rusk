// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use core::iter::Extend;
use core::mem;

use dusk_abi::ContractId;
use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::BlsScalar;
use dusk_pki::{Ownable, SecretSpendKey};
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge::hash;
use dusk_schnorr::Proof;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

const CONTRACT_ID_SIZE: usize = mem::size_of::<ContractId>();

/// The structure sent over the network representing a transaction.
#[derive(Debug, Clone)]
pub struct Transaction {
    inputs: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    fee: Fee,
    proof: Proof,
    crossover: Option<Crossover>,
    call: Option<(ContractId, Vec<u8>)>,
}

impl Transaction {
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

    /// Returns the hash of the transaction without the proof.
    pub fn hash(&self) -> BlsScalar {
        let clone = self.clone();
        ProvableTransaction::from(clone).hash()
    }

    /// Return the internal representation of scalars to be hashed.
    pub fn hash_inputs(&self) -> Vec<BlsScalar> {
        let clone = self.clone();
        ProvableTransaction::from(clone).hash_inputs()
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

/// A transaction that is yet to be proven.
pub(crate) struct ProvableTransaction {
    inputs: Vec<BlsScalar>,
    outputs: Vec<Note>,
    anchor: BlsScalar,
    fee: Fee,
    crossover: Option<Crossover>,
    call: Option<(ContractId, Vec<u8>)>,
}

impl From<Transaction> for ProvableTransaction {
    fn from(tx: Transaction) -> Self {
        Self {
            anchor: tx.anchor,
            call: tx.call,
            crossover: tx.crossover,
            fee: tx.fee,
            outputs: tx.outputs,
            inputs: tx.inputs,
        }
    }
}

impl ProvableTransaction {
    /// Instantiates a new unproven transaction.
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

    /// Consumes this unproven transaction, proves it and returns a
    /// [`Transaction`].
    pub(crate) fn prove<Rng: RngCore + CryptoRng>(
        self,
        ssk: &SecretSpendKey,
    ) -> Transaction {
        todo!()
    }

    /// Hash the unproven transaction.
    pub(crate) fn hash(&self) -> BlsScalar {
        hash(&self.hash_inputs())
    }

    fn hash_inputs(&self) -> Vec<BlsScalar> {
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
                    8 + cdata.len() / BlsScalar::SIZE
                        + if cdata.len() % BlsScalar::SIZE == 0 { 0 } else { 1 }
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

/// Returns hash inputs from a slice of bytes, padding to zero.
fn hash_inputs_from_bytes<B: AsRef<[u8]>>(bytes: B) -> Vec<BlsScalar> {
    let padded = {
        let mut buf = bytes.as_ref().to_vec();
        let padding = vec![0; buf.len() % BlsScalar::SIZE];

        buf.extend(padding);

        buf
    };

    padded
        .chunks(BlsScalar::SIZE)
        .map(|c| {
            // Unwrapping here is ok because we've padded the last chunk to the
            // correct size
            BlsScalar::from_slice(c).unwrap()
        })
        .collect()
}

// Will become superfluous redundant
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

#[cfg(test)]
mod tests {
    use super::*;
    use dusk_jubjub::JubJubScalar;
    use dusk_pki::{Ownable, SecretSpendKey};

    #[test]
    fn serde() {
        let mut rng = rand::thread_rng();
        let ssk = SecretSpendKey::random(&mut rng);
        let psk = ssk.public_spend_key();
        let blinding_factor = JubJubScalar::random(&mut rng);

        let inputs = vec![BlsScalar::from(1), BlsScalar::from(2)];
        let outputs =
            vec![Note::obfuscated(&mut rng, &psk, 42, blinding_factor)];
        let anchor = BlsScalar::random(&mut rng);
        let fee = Fee::new(&mut rng, 42, 24, &psk);
        let crossover = None;
        let call = Some((ContractId::from([1u8; 32]), vec![1, 2, 3, 4]));
        let proof = Proof::new(
            &ssk.sk_r(outputs[0].stealth_address()),
            &mut rng,
            anchor,
        );

        let tx = Transaction {
            inputs: inputs.clone(),
            outputs: outputs.clone(),
            anchor,
            fee,
            crossover,
            call: call.clone(),
            proof,
        };

        let serde_tx = Transaction::from_bytes(
            tx.to_var_bytes().expect("serializing to go ok"),
        )
        .expect("serialized to be deserializable");

        assert_eq!(inputs, serde_tx.inputs);
        assert_eq!(outputs, serde_tx.outputs);
        assert_eq!(anchor, serde_tx.anchor);
        assert_eq!(fee, serde_tx.fee);
        assert_eq!(crossover, serde_tx.crossover);
        assert_eq!(call, serde_tx.call);
        assert_eq!(proof.to_bytes(), serde_tx.proof.to_bytes());
    }
}
