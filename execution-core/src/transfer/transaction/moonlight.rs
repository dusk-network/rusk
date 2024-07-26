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
    transfer::{Bytecode, ContractCall, ContractDeploy, ContractExec},
    BlsPublicKey, BlsScalar, BlsSignature,
};

/// Moonlight transaction.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transaction {
    pub(crate) payload: Payload,
    pub(crate) signature: BlsSignature,
}

impl Transaction {
    /// Create a new transaction.
    #[must_use]
    pub fn new(payload: Payload, signature: BlsSignature) -> Self {
        Self { payload, signature }
    }

    /// The payload of the transaction.
    #[must_use]
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// The proof of the transaction.
    #[must_use]
    pub fn signature(&self) -> &BlsSignature {
        &self.signature
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
                from: self.payload.from,
                to: self.payload.to,
                value: self.payload.value,
                deposit: self.payload.deposit,
                gas_limit: self.payload.gas_limit,
                gas_price: self.payload.gas_price,
                nonce: self.payload.nonce,
                exec: Some(ContractExec::Deploy(ContractDeploy {
                    owner: deploy.owner.clone(),
                    constructor_args: deploy.constructor_args.clone(),
                    bytecode: Bytecode {
                        hash: deploy.bytecode.hash,
                        bytes: Vec::new(),
                    },
                    nonce: deploy.nonce,
                })),
            },
            *self.signature(),
        ))
    }

    /// Serialize a transaction into a byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let payload_bytes = self.payload.to_var_bytes();
        bytes.extend((payload_bytes.len() as u64).to_bytes());
        bytes.extend(payload_bytes);

        bytes.extend(self.signature.to_bytes());

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

        let signature = BlsSignature::from_bytes(
            buf.try_into().map_err(|_| BytesError::InvalidData)?,
        )
        .map_err(|_| BytesError::InvalidData)?;

        Ok(Self { payload, signature })
    }

    /// Return input bytes to hash the payload.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the transaction again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = self.payload.to_hash_input_bytes();
        bytes.extend(self.signature.to_bytes());
        bytes
    }

    /// Return the message that is meant to be signed over to make the
    /// transaction a valid one.
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        self.payload.to_hash_input_bytes()
    }

    /// Create the payload hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }
}

/// The payload for a moonlight transaction.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Payload {
    /// Key of the sender of this transaction.
    pub from: BlsPublicKey,
    /// Key of the receiver of the funds.
    pub to: Option<BlsPublicKey>,
    /// Value to be transferred.
    pub value: u64,
    /// Deposit for a contract.
    pub deposit: u64,
    /// Limit on the gas to be spent.
    pub gas_limit: u64,
    /// Price for each unit of gas.
    pub gas_price: u64,
    /// Nonce used for replay protection. Nonces are strictly increasing,
    /// meaning that once a transaction has been settled, only a higher
    /// nonce can be used.
    ///
    /// The current nonce is queryable via the transfer contract and best
    /// practice is to use `nonce + 1` for a single transaction.
    pub nonce: u64,
    /// Data to do a contract call or deployment.
    pub exec: Option<ContractExec>,
}

impl Payload {
    /// Serialize the payload into a byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.from.to_bytes());

        // serialize the recipient
        match self.to {
            Some(to) => {
                bytes.push(1);
                bytes.extend(to.to_bytes());
            }
            None => {
                bytes.push(0);
            }
        }

        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.gas_limit.to_bytes());
        bytes.extend(self.gas_price.to_bytes());
        bytes.extend(self.nonce.to_bytes());

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

    /// Deserialize the payload from bytes slice.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let from = BlsPublicKey::from_reader(&mut buf)?;

        // deserialize recipient
        let to = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(BlsPublicKey::from_reader(&mut buf)?),
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        let value = u64::from_reader(&mut buf)?;
        let deposit = u64::from_reader(&mut buf)?;
        let gas_limit = u64::from_reader(&mut buf)?;
        let gas_price = u64::from_reader(&mut buf)?;
        let nonce = u64::from_reader(&mut buf)?;

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
            from,
            to,
            value,
            deposit,
            gas_limit,
            gas_price,
            nonce,
            exec,
        })
    }

    /// Return input bytes to hash the payload.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the payload again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.from.to_bytes());
        if let Some(to) = &self.to {
            bytes.extend(to.to_bytes());
        }
        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.gas_limit.to_bytes());
        bytes.extend(self.gas_price.to_bytes());
        bytes.extend(self.nonce.to_bytes());

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

    /// Create the payload hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }
}
