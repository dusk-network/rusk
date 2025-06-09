// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to the moonlight transaction model of Dusk's transfer
//! contract.

#[cfg(feature = "serde")]
use serde_with::{serde_as, DisplayFromStr};

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use rkyv::{Archive, Deserialize, Serialize};

use crate::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
    Signature as AccountSignature,
};
use crate::transfer::data::{
    ContractBytecode, ContractCall, ContractDeploy, TransactionData,
    MAX_MEMO_SIZE,
};
use crate::{BlsScalar, Error};

/// A Moonlight account's information.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[cfg_attr(feature = "serde", cfg_eval, serde_as)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountData {
    /// Number used for replay protection.
    #[cfg_attr(feature = "serde", serde_as(as = "DisplayFromStr"))]
    pub nonce: u64,
    /// Account balance.
    #[cfg_attr(feature = "serde", serde_as(as = "DisplayFromStr"))]
    pub balance: u64,
}

/// Moonlight transaction.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transaction {
    payload: Payload,
    signature: AccountSignature,
}

impl Transaction {
    /// Create a new transaction.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - the memo, if given, is too large
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sender_sk: &AccountSecretKey,
        receiver: Option<AccountPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Self, Error> {
        let refund_address = AccountPublicKey::from(sender_sk);

        Self::new_with_refund(
            sender_sk,
            &refund_address,
            receiver,
            value,
            deposit,
            gas_limit,
            gas_price,
            nonce,
            chain_id,
            data,
        )
    }

    /// Create a new transaction with a specified refund-address for the gas
    /// refund.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - the memo, if given, is too large
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_refund(
        sender_sk: &AccountSecretKey,
        refund_pk: &AccountPublicKey,
        receiver: Option<AccountPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Self, Error> {
        let data = data.map(Into::into);
        let sender = AccountPublicKey::from(sender_sk);
        let receiver = receiver.unwrap_or(sender);

        let fee = Fee {
            gas_limit,
            gas_price,
            refund_address: *refund_pk,
        };

        let payload = Payload {
            chain_id,
            sender,
            receiver,
            value,
            deposit,
            fee,
            nonce,
            data,
        };

        Self::sign_payload(sender_sk, payload)
    }

    /// Create a transaction by signing a previously generated payload with a
    /// given secret-key.
    ///
    /// Note that this transaction will be invalid if the secret-key used for
    /// signing doesn't form a valid key-pair with the public-key of the
    /// `sender`.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - the payload memo, if given, is too large
    pub fn sign_payload(
        sender_sk: &AccountSecretKey,
        payload: Payload,
    ) -> Result<Self, Error> {
        if let Some(TransactionData::Memo(memo)) = payload.data.as_ref() {
            if memo.len() > MAX_MEMO_SIZE {
                return Err(Error::MemoTooLarge(memo.len()));
            }
        }

        let digest = payload.signature_message();
        let signature = sender_sk.sign(&digest);

        Ok(Self { payload, signature })
    }

    /// The proof of the transaction.
    #[must_use]
    pub fn signature(&self) -> &AccountSignature {
        &self.signature
    }

    /// Return the sender of the transaction.
    #[must_use]
    pub fn sender(&self) -> &AccountPublicKey {
        &self.payload.sender
    }

    /// Return the address to send the transaction refund to.
    #[must_use]
    pub fn refund_address(&self) -> &AccountPublicKey {
        &self.payload.fee.refund_address
    }

    /// Return the receiver of the transaction if it's different from the
    /// sender. Otherwise, return None.
    #[must_use]
    pub fn receiver(&self) -> Option<&AccountPublicKey> {
        if self.payload.sender == self.payload.receiver {
            None
        } else {
            Some(&self.payload.receiver)
        }
    }

    /// Return the value transferred in the transaction.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.payload.value
    }

    /// Returns the deposit of the transaction.
    #[must_use]
    pub fn deposit(&self) -> u64 {
        self.payload.deposit
    }

    /// Returns the gas limit of the transaction.
    #[must_use]
    pub fn gas_limit(&self) -> u64 {
        self.payload.fee.gas_limit
    }

    /// Returns the gas price of the transaction.
    #[must_use]
    pub fn gas_price(&self) -> u64 {
        self.payload.fee.gas_price
    }

    /// Returns the nonce of the transaction.
    #[must_use]
    pub fn nonce(&self) -> u64 {
        self.payload.nonce
    }

    /// Returns the chain ID of the transaction.
    #[must_use]
    pub fn chain_id(&self) -> u8 {
        self.payload.chain_id
    }

    /// Return the contract call data, if there is any.
    #[must_use]
    pub fn call(&self) -> Option<&ContractCall> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.data()? {
            TransactionData::Call(ref c) => Some(c),
            _ => None,
        }
    }

    /// Return the contract deploy data, if there is any.
    #[must_use]
    pub fn deploy(&self) -> Option<&ContractDeploy> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.data()? {
            TransactionData::Deploy(ref d) => Some(d),
            _ => None,
        }
    }

    /// Returns the memo used with the transaction, if any.
    #[must_use]
    pub fn memo(&self) -> Option<&[u8]> {
        match self.data()? {
            TransactionData::Memo(memo) => Some(memo),
            _ => None,
        }
    }

    /// Returns the transaction data, if it exists.
    #[must_use]
    pub fn data(&self) -> Option<&TransactionData> {
        self.payload.data.as_ref()
    }

    /// Creates a modified clone of this transaction if it contains data for
    /// deployment, clones all fields except for the bytecode 'bytes' part.
    /// Returns none if the transaction is not a deployment transaction.
    #[must_use]
    pub fn strip_off_bytecode(&self) -> Option<Self> {
        let deploy = self.deploy()?;

        let stripped_deploy = TransactionData::Deploy(ContractDeploy {
            owner: deploy.owner.clone(),
            init_args: deploy.init_args.clone(),
            bytecode: ContractBytecode {
                hash: deploy.bytecode.hash,
                bytes: Vec::new(),
            },
            nonce: deploy.nonce,
        });

        let mut stripped_transaction = self.clone();
        stripped_transaction.payload.data = Some(stripped_deploy);

        Some(stripped_transaction)
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

        let signature = AccountSignature::from_bytes(
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
        let mut bytes = self.payload.signature_message();
        bytes.extend(self.signature.to_bytes());
        bytes
    }

    /// Return the message that is meant to be signed over to make the
    /// transaction a valid one.
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        self.payload.signature_message()
    }

    /// Create the transaction hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }
}

/// The payload for a moonlight transaction.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Payload {
    /// ID of the chain for this transaction to execute on.
    pub chain_id: u8,
    /// Key of the sender of this transaction.
    pub sender: AccountPublicKey,
    /// Key of the receiver of the funds.
    pub receiver: AccountPublicKey,
    /// Value to be transferred.
    pub value: u64,
    /// Deposit for a contract.
    pub deposit: u64,
    /// Data used to calculate the transaction fee and refund unspent gas.
    pub fee: Fee,
    /// Nonce used for replay protection. Moonlight nonces are strictly
    /// increasing and incremental, meaning that for a transaction to be
    /// valid, only the current nonce + 1 can be used.
    ///
    /// The current nonce is queryable via the transfer contract.
    pub nonce: u64,
    /// Data to do a contract call, deployment, or insert a memo.
    pub data: Option<TransactionData>,
}

impl Payload {
    /// Serialize the payload into a byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::from([self.chain_id]);

        bytes.extend(self.sender.to_bytes());
        // to save space we only serialize the receiver if it's different from
        // the sender
        if self.sender == self.receiver {
            bytes.push(0);
        } else {
            bytes.push(1);
            bytes.extend(self.receiver.to_bytes());
        }

        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());

        // serialize the fee
        bytes.extend(self.fee.gas_limit.to_bytes());
        bytes.extend(self.fee.gas_price.to_bytes());
        // to save space we only serialize the refund-address if it's different
        // from the sender
        if self.sender == self.fee.refund_address {
            bytes.push(0);
        } else {
            bytes.push(1);
            bytes.extend(self.fee.refund_address.to_bytes());
        }

        bytes.extend(self.nonce.to_bytes());

        // serialize the contract call, deployment or memo, if present.
        match &self.data {
            Some(TransactionData::Call(call)) => {
                bytes.push(1);
                bytes.extend(call.to_var_bytes());
            }
            Some(TransactionData::Deploy(deploy)) => {
                bytes.push(2);
                bytes.extend(deploy.to_var_bytes());
            }
            Some(TransactionData::Memo(memo)) => {
                bytes.push(3);
                bytes.extend((memo.len() as u64).to_bytes());
                bytes.extend(memo);
            }
            Some(TransactionData::Blob(_, _)) => {
                todo!("Not implemented yet");
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

        let chain_id = u8::from_reader(&mut buf)?;
        let sender = AccountPublicKey::from_reader(&mut buf)?;
        let receiver = match u8::from_reader(&mut buf)? {
            0 => sender,
            1 => AccountPublicKey::from_reader(&mut buf)?,
            _ => {
                return Err(BytesError::InvalidData);
            }
        };
        let value = u64::from_reader(&mut buf)?;
        let deposit = u64::from_reader(&mut buf)?;

        // deserialize the fee
        let gas_limit = u64::from_reader(&mut buf)?;
        let gas_price = u64::from_reader(&mut buf)?;
        let refund_address = match u8::from_reader(&mut buf)? {
            0 => sender,
            1 => AccountPublicKey::from_reader(&mut buf)?,
            _ => {
                return Err(BytesError::InvalidData);
            }
        };
        let fee = Fee {
            gas_limit,
            gas_price,
            refund_address,
        };

        let nonce = u64::from_reader(&mut buf)?;

        // deserialize contract call, deploy data, or memo, if present
        let data = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(TransactionData::Call(ContractCall::from_slice(buf)?)),
            2 => {
                Some(TransactionData::Deploy(ContractDeploy::from_slice(buf)?))
            }
            3 => {
                // we only build for 64-bit so this truncation is impossible
                #[allow(clippy::cast_possible_truncation)]
                let size = u64::from_reader(&mut buf)? as usize;

                if buf.len() != size || size > MAX_MEMO_SIZE {
                    return Err(BytesError::InvalidData);
                }

                let memo = buf[..size].to_vec();
                Some(TransactionData::Memo(memo))
            }
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        Ok(Self {
            chain_id,
            sender,
            receiver,
            value,
            deposit,
            fee,
            nonce,
            data,
        })
    }

    /// Return input bytes to hash the payload.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the payload again.
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        let mut bytes = Vec::from([self.chain_id]);

        bytes.extend(self.sender.to_bytes());
        if self.receiver != self.sender {
            bytes.extend(self.receiver.to_bytes());
        }
        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.fee.gas_limit.to_bytes());
        bytes.extend(self.fee.gas_price.to_bytes());
        if self.fee.refund_address != self.sender {
            bytes.extend(self.fee.refund_address.to_bytes());
        }
        bytes.extend(self.nonce.to_bytes());

        match &self.data {
            Some(TransactionData::Deploy(d)) => {
                bytes.extend(&d.bytecode.to_hash_input_bytes());
                bytes.extend(&d.owner);
                if let Some(init_args) = &d.init_args {
                    bytes.extend(init_args);
                }
            }
            Some(TransactionData::Call(c)) => {
                bytes.extend(c.contract.as_bytes());
                bytes.extend(c.fn_name.as_bytes());
                bytes.extend(&c.fn_args);
            }
            Some(TransactionData::Memo(m)) => {
                bytes.extend(m);
            }
            Some(TransactionData::Blob(_, _)) => {
                todo!("Not implemented yet");
            }
            None => {}
        }

        bytes
    }

    /// Temporarily solution to create a signature message for the test.
    #[must_use]
    pub fn new_signature_message(&self) -> Vec<u8> {
        let mut bytes = Vec::from([self.chain_id]);

        bytes.extend(self.sender.to_bytes());
        if self.receiver != self.sender {
            bytes.extend(self.receiver.to_bytes());
        }
        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.fee.gas_limit.to_bytes());
        bytes.extend(self.fee.gas_price.to_bytes());
        if self.fee.refund_address != self.sender {
            bytes.extend(self.fee.refund_address.to_bytes());
        }
        bytes.extend(self.nonce.to_bytes());

        // Convert TransactionData::Blob to TransactionData::Memo for signature
        // message
        let data = match &self.data {
            // Some(TransactionData::Blob(hashes, _)) => {
            //     Some(TransactionData::Memo(hashes.clone()))
            // }
            other => other.clone(),
        };

        match data {
            Some(TransactionData::Deploy(d)) => {
                bytes.extend(&d.bytecode.to_hash_input_bytes());
                bytes.extend(&d.owner);
                if let Some(init_args) = &d.init_args {
                    bytes.extend(init_args);
                }
            }
            Some(TransactionData::Call(c)) => {
                bytes.extend(c.contract.as_bytes());
                bytes.extend(c.fn_name.as_bytes());
                bytes.extend(&c.fn_args);
            }
            Some(TransactionData::Memo(m)) => {
                bytes.extend(m);
            }
            Some(TransactionData::Blob(hashes, _)) => {
                bytes.extend(hashes);
            }
            _ => {}
        }

        bytes
    }
}

/// The Fee structure
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Fee {
    /// Limit on the gas to be spent.
    pub gas_limit: u64,
    /// Price for each unit of gas.
    pub gas_price: u64,
    /// Address to which to refund the unspent gas.
    pub refund_address: AccountPublicKey,
}
