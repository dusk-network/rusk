// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    transfer::{Bytecode, ContractCall, ContractDeploy, ContractExec, Fee},
    BlsScalar, JubJubAffine, Sender, TxSkeleton,
};

/// Phoenix transaction.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transaction {
    pub(crate) payload: Payload,
    pub(crate) proof: Vec<u8>,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Transaction {}

impl Transaction {
    /// Create a new transaction.
    #[must_use]
    pub fn new(payload: Payload, proof: impl Into<Vec<u8>>) -> Self {
        Self {
            payload,
            proof: proof.into(),
        }
    }

    /// The payload of the transaction.
    #[must_use]
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// The proof of the transaction.
    #[must_use]
    pub fn proof(&self) -> &[u8] {
        &self.proof
    }

    /// Return the contract call data, if there is any.
    #[must_use]
    pub fn call(&self) -> Option<&ContractCall> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.exec()? {
            ContractExec::Call(ref c) => Some(c),
            _ => None,
        }
    }

    /// Return the contract deploy data, if there is any.
    #[must_use]
    pub fn deploy(&self) -> Option<&ContractDeploy> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.exec()? {
            ContractExec::Deploy(ref d) => Some(d),
            _ => None,
        }
    }

    /// Returns the contract execution, if it exists.
    #[must_use]
    fn exec(&self) -> Option<&ContractExec> {
        self.payload.exec.as_ref()
    }

    /// Creates a modified clone of this transaction if it contains data for
    /// deployment, clones all fields except for the bytecode' 'bytes' part.
    /// Returns none if the transaction is not a deployment transaction.
    #[must_use]
    pub fn strip_off_bytecode(&self) -> Option<Self> {
        let deploy = self.deploy()?;

        Some(Self::new(
            Payload {
                tx_skeleton: self.payload().tx_skeleton.clone(),
                fee: self.payload().fee,
                exec: Some(ContractExec::Deploy(ContractDeploy {
                    owner: deploy.owner.clone(),
                    constructor_args: deploy.constructor_args.clone(),
                    bytecode: Bytecode {
                        hash: deploy.bytecode.hash,
                        bytes: Vec::new(),
                    },
                })),
            },
            self.proof(),
        ))
    }

    /// Serialize the `Transaction` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let payload_bytes = self.payload.to_var_bytes();
        bytes.extend((payload_bytes.len() as u64).to_bytes());
        bytes.extend(payload_bytes);

        bytes.extend((self.proof.len() as u64).to_bytes());
        bytes.extend(&self.proof);

        bytes
    }

    /// Deserialize the Transaction from a bytes buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let payload_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;

        if buf.len() < payload_len {
            return Err(BytesError::InvalidData);
        }
        let (payload_buf, new_buf) = buf.split_at(payload_len);

        let payload = Payload::from_slice(payload_buf)?;
        buf = new_buf;

        let proof_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;
        let proof = buf[..proof_len].into();

        Ok(Self { payload, proof })
    }

    /// Return input bytes to hash the Transaction.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the `Transaction` again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = self.payload.to_hash_input_bytes();
        bytes.extend(&self.proof);
        bytes
    }

    /// Create the `Transaction`-hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }

    /// Return the public input to be used in the phoenix-transaction circuit
    /// verification
    ///
    /// These are:
    /// - `payload_hash`
    /// - `root`
    /// - `[nullifier; I]`
    /// - `[output_value_commitment; 2]`
    /// - `max_fee`
    /// - `deposit`
    /// - `(npk_0, npk_1)`
    /// - `(enc_A_npk_0, enc_B_npk_0)`
    /// - `(enc_A_npk_1, enc_B_npk_1)`
    ///
    /// # Panics
    /// Panics if one of the output-notes doesn't have the sender set to
    /// [`Sender::Encryption`].
    #[must_use]
    pub fn public_inputs(&self) -> Vec<BlsScalar> {
        let tx_skeleton = &self.payload.tx_skeleton;

        // retrieve the number of input and output notes
        let input_len = tx_skeleton.nullifiers.len();
        let output_len = tx_skeleton.outputs.len();

        let size =
            // payload-hash and root
            1 + 1
            // nullifiers
            + input_len
            // output-notes value-commitment
            + 2 * output_len
            // max-fee and deposit
            + 1 + 1
            // output-notes public-keys
            + 2 * output_len
            // sender-encryption for both output-notes
            + 2 * 4 * output_len;
        // build the public input vector
        let mut pis = Vec::<BlsScalar>::with_capacity(size);
        pis.push(self.payload.hash());
        pis.push(tx_skeleton.root);
        pis.extend(tx_skeleton.nullifiers().iter());
        tx_skeleton.outputs().iter().for_each(|note| {
            let value_commitment = note.value_commitment();
            pis.push(value_commitment.get_u());
            pis.push(value_commitment.get_v());
        });
        pis.push(tx_skeleton.max_fee().into());
        pis.push(tx_skeleton.deposit().into());
        tx_skeleton.outputs().iter().for_each(|note| {
            let note_pk =
                JubJubAffine::from(note.stealth_address().note_pk().as_ref());
            pis.push(note_pk.get_u());
            pis.push(note_pk.get_v());
        });
        tx_skeleton.outputs().iter().for_each(|note| {
            match note.sender() {
                Sender::Encryption(sender_enc) => {
                    pis.push(sender_enc[0].0.get_u());
                    pis.push(sender_enc[0].0.get_v());
                    pis.push(sender_enc[0].1.get_u());
                    pis.push(sender_enc[0].1.get_v());
                    pis.push(sender_enc[1].0.get_u());
                    pis.push(sender_enc[1].0.get_v());
                    pis.push(sender_enc[1].1.get_u());
                    pis.push(sender_enc[1].1.get_v());
                }
                Sender::ContractInfo(_) => {
                    panic!("All output-notes must provide a sender-encryption")
                }
            };
        });

        pis
    }
}

/// The transaction payload
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Payload {
    /// Transaction skeleton used for the phoenix transaction.
    pub tx_skeleton: TxSkeleton,
    /// Data used to calculate the transaction fee.
    pub fee: Fee,
    /// Data to do a contract call or deployment.
    pub exec: Option<ContractExec>,
}

impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Payload {}

impl Payload {
    /// Serialize the `Payload` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // serialize the tx-skeleton
        let skeleton_bytes = self.tx_skeleton.to_var_bytes();
        bytes.extend((skeleton_bytes.len() as u64).to_bytes());
        bytes.extend(skeleton_bytes);

        // serialize the fee
        bytes.extend(self.fee.to_bytes());

        // serialize the contract call/deployment
        match &self.exec {
            Some(ContractExec::Deploy(deploy)) => {
                bytes.push(2);
                bytes.extend(deploy.to_var_bytes());
            }
            Some(ContractExec::Call(call)) => {
                bytes.push(1);
                bytes.extend(call.to_var_bytes());
            }
            _ => bytes.push(0),
        }

        bytes
    }

    /// Deserialize the Payload from a bytes buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        // deserialize the tx-skeleton
        #[allow(clippy::cast_possible_truncation)]
        let skeleton_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;
        let tx_skeleton = TxSkeleton::from_slice(buf)?;
        buf = &buf[skeleton_len..];

        // deserialize fee
        let fee = Fee::from_reader(&mut buf)?;

        // deserialize contract call/deploy data
        let exec = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(ContractExec::Call(ContractCall::from_slice(buf)?)),
            2 => Some(ContractExec::Deploy(ContractDeploy::from_slice(buf)?)),
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        Ok(Self {
            tx_skeleton,
            fee,
            exec,
        })
    }

    /// Return input bytes to hash the payload.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the `Payload` again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = self.tx_skeleton.to_hash_input_bytes();

        match &self.exec {
            Some(ContractExec::Deploy(d)) => {
                bytes.extend(&d.bytecode.to_hash_input_bytes());
                bytes.extend(&d.owner);
                if let Some(constructor_args) = &d.constructor_args {
                    bytes.extend(constructor_args);
                }
            }
            Some(ContractExec::Call(c)) => {
                bytes.extend(c.contract);
                bytes.extend(c.fn_name.as_bytes());
                bytes.extend(&c.fn_args);
            }
            _ => {}
        }

        bytes
    }

    /// Create the `Payload`-hash to be used as an input to the
    /// pheonix-transaction circuit.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }
}
