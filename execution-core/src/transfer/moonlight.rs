// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to the moonlight transaction model of Dusk's transfer
//! contract.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    signatures::bls::{
        PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
        Signature as AccountSignature,
    },
    transfer::data::{
        ContractBytecode, ContractCall, ContractDeploy, TransactionData,
        MAX_MEMO_SIZE,
    },
    BlsScalar, Error,
};

/// A Moonlight account's information.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct AccountData {
    /// Number used for replay protection.
    pub nonce: u64,
    /// Account balance.
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
        from_sk: &AccountSecretKey,
        to_account: Option<AccountPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Self, Error> {
        let data = data.map(Into::into);

        if let Some(TransactionData::Memo(memo)) = data.as_ref() {
            if memo.len() > MAX_MEMO_SIZE {
                return Err(Error::MemoTooLarge(memo.len()));
            }
        }

        let payload = Payload {
            chain_id,
            from_account: AccountPublicKey::from(from_sk),
            to_account,
            value,
            deposit,
            gas_limit,
            gas_price,
            nonce,
            data,
        };

        let digest = payload.signature_message();
        let signature = from_sk.sign(&digest);

        Ok(Self { payload, signature })
    }

    /// The proof of the transaction.
    #[must_use]
    pub fn signature(&self) -> &AccountSignature {
        &self.signature
    }

    /// Return the sender of the transaction.
    #[must_use]
    pub fn from_account(&self) -> &AccountPublicKey {
        &self.payload.from_account
    }

    /// Return the receiver of the transaction, if it exists.
    #[must_use]
    pub fn to_account(&self) -> Option<&AccountPublicKey> {
        self.payload.to_account.as_ref()
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
        self.payload.gas_limit
    }

    /// Returns the gas price of the transaction.
    #[must_use]
    pub fn gas_price(&self) -> u64 {
        self.payload.gas_price
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
    fn data(&self) -> Option<&TransactionData> {
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
            constructor_args: deploy.constructor_args.clone(),
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
struct Payload {
    /// ID of the chain for this transaction to execute on.
    pub chain_id: u8,
    /// Key of the sender of this transaction.
    pub from_account: AccountPublicKey,
    /// Key of the receiver of the funds.
    pub to_account: Option<AccountPublicKey>,
    /// Value to be transferred.
    pub value: u64,
    /// Deposit for a contract.
    pub deposit: u64,
    /// Limit on the gas to be spent.
    pub gas_limit: u64,
    /// Price for each unit of gas.
    pub gas_price: u64,
    /// Nonce used for replay protection. Nonces are strictly increasing and
    /// incremental, meaning that for a transaction to be valid, only the
    /// current nonce + 1 can be used.
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

        bytes.extend(self.from_account.to_bytes());

        // serialize the recipient
        match self.to_account {
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

        let from_account = AccountPublicKey::from_reader(&mut buf)?;

        // deserialize recipient
        let to_account = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(AccountPublicKey::from_reader(&mut buf)?),
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        let value = u64::from_reader(&mut buf)?;
        let deposit = u64::from_reader(&mut buf)?;
        let gas_limit = u64::from_reader(&mut buf)?;
        let gas_price = u64::from_reader(&mut buf)?;
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
            from_account,
            to_account,
            value,
            deposit,
            gas_limit,
            gas_price,
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

        bytes.extend(self.from_account.to_bytes());
        if let Some(to) = &self.to_account {
            bytes.extend(to.to_bytes());
        }
        bytes.extend(self.value.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.gas_limit.to_bytes());
        bytes.extend(self.gas_price.to_bytes());
        bytes.extend(self.nonce.to_bytes());

        match &self.data {
            Some(TransactionData::Deploy(d)) => {
                bytes.extend(&d.bytecode.to_hash_input_bytes());
                bytes.extend(&d.owner);
                if let Some(constructor_args) = &d.constructor_args {
                    bytes.extend(constructor_args);
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
            None => {}
        }

        bytes
    }
}
