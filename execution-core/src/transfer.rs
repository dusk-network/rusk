// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to Dusk's transfer contract that are shared across the
//! network.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use core::cmp;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use dusk_poseidon::{Domain, Hash};
use ff::Field;
use rand::{CryptoRng, RngCore};
use rkyv::{
    ser::serializers::AllocSerializer, Archive, Deserialize, Fallible,
    Serialize,
};

use crate::{
    BlsPublicKey, BlsScalar, JubJubAffine, JubJubScalar, Note, PublicKey,
    Sender, StealthAddress,
};

mod transaction;
use crate::bytecode::Bytecode;
use crate::reader::{read_arr, read_str, read_vec};
pub use transaction::{Payload, Transaction};

/// Unique ID to identify a contract.
pub type ContractId = [u8; 32];

/// The depth of the transfer tree.
pub const TRANSFER_TREE_DEPTH: usize = 17;

/// A leaf of the transfer tree.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TreeLeaf {
    /// The height of the block when the note was inserted in the tree.
    pub block_height: u64,
    /// The note inserted in the tree.
    pub note: Note,
}

/// Data to mint a new phoenix-note with a given value to a stealth address.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Mint {
    /// The address to mint to.
    pub address: StealthAddress,
    /// The value to mint to the address.
    pub value: u64,
    /// The account that sent the `mint` request.
    pub sender: BlsPublicKey,
}

impl Serializable<{ StealthAddress::SIZE + u64::SIZE + BlsPublicKey::SIZE }>
    for Mint
{
    type Error = BytesError;

    /// Converts a Fee into it's byte representation
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[..StealthAddress::SIZE].copy_from_slice(&self.address.to_bytes());
        let mut start = StealthAddress::SIZE;
        buf[start..start + u64::SIZE].copy_from_slice(&self.value.to_bytes());
        start += u64::SIZE;
        buf[start..start + BlsPublicKey::SIZE]
            .copy_from_slice(&self.sender.to_bytes());
        buf
    }

    /// Attempts to convert a byte representation of a fee into a `Fee`,
    /// failing if the input is invalid
    fn from_bytes(bytes: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut buf = &bytes[..];
        let address = StealthAddress::from_reader(&mut buf)?;
        let value = u64::from_reader(&mut buf)?;
        let sender = BlsPublicKey::from_reader(&mut buf)?;

        Ok(Mint {
            address,
            value,
            sender,
        })
    }
}

/// Data for either contract call or contract deployment.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum CallOrDeploy {
    /// Data for a contract call.
    Call(ContractCall),
    /// Data for a contract deployment.
    Deploy(ContractDeploy),
}

/// Data for performing a contract deployment
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractDeploy {
    /// Bytecode of the contract to be deployed.
    pub bytecode: Bytecode,
    /// Owner of the contract to be deployed.
    pub owner: Vec<u8>,
    /// Constructor arguments of the deployed contract.
    pub constructor_args: Option<Vec<u8>>,
}

/// All the data the transfer-contract needs to perform a contract-call.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractCall {
    /// The unique ID of the contract to be called.
    pub contract: ContractId,
    /// The function of the contract that should be called.
    pub fn_name: String,
    /// The function arguments for the contract call, in bytes.
    pub fn_args: Vec<u8>,
}

// The size of the argument buffer in bytes as specified by piecrust-uplink
const ARGBUF_LEN: usize = 64 * 1024;

impl ContractDeploy {
    /// Serialize a `ContractDeploy` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(&self.bytecode.to_var_bytes());

        bytes.extend((self.owner.len() as u64).to_bytes());
        bytes.extend(&self.owner);

        match &self.constructor_args {
            Some(constructor_args) => {
                bytes.push(1);
                bytes.extend((constructor_args.len() as u64).to_bytes());
                bytes.extend(constructor_args);
            }
            None => bytes.push(0),
        }

        bytes
    }

    /// Deserialize a `ContractDeploy` from a byte buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let bytecode = Bytecode::from_buf(&mut buf)?;

        let owner = read_vec(&mut buf)?;

        let constructor_args = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(read_vec(&mut buf)?),
            _ => return Err(BytesError::InvalidData),
        };

        Ok(Self {
            bytecode,
            owner,
            constructor_args,
        })
    }
}

impl ContractCall {
    /// Creates a new contract call.
    ///
    /// # Errors
    /// Errors if rkyv serialization fails.
    pub fn new(
        contract: impl Into<[u8; 32]>,
        fn_name: impl Into<String>,
        fn_args: &impl Serialize<AllocSerializer<ARGBUF_LEN>>,
    ) -> Result<Self, <AllocSerializer<ARGBUF_LEN> as Fallible>::Error> {
        Ok(Self {
            contract: contract.into(),
            fn_name: fn_name.into(),
            fn_args: rkyv::to_bytes::<_, ARGBUF_LEN>(fn_args)?.to_vec(),
        })
    }

    /// Serialize a `ContractCall` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.contract);

        let fn_name_bytes = self.fn_name.as_bytes();
        bytes.extend((fn_name_bytes.len() as u64).to_bytes());
        bytes.extend(fn_name_bytes);

        bytes.extend((self.fn_args.len() as u64).to_bytes());
        bytes.extend(&self.fn_args);

        bytes
    }

    /// Deserialize a `ContractCall` from a byte buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not cannonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let contract = read_arr::<32>(&mut buf)?;

        let fn_name = read_str(&mut buf)?;

        let fn_args = read_vec(&mut buf)?;

        Ok(Self {
            contract,
            fn_name,
            fn_args,
        })
    }
}

/// The Fee structure
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Fee {
    /// Gas limit set for a phoenix transaction
    pub gas_limit: u64,
    /// Gas price set for a phoenix transaction
    pub gas_price: u64,
    /// Address to send the remainder note
    pub stealth_address: StealthAddress,
    /// Sender to use for the remainder
    pub sender: Sender,
}

impl PartialEq for Fee {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender && self.hash() == other.hash()
    }
}

impl Eq for Fee {}

impl Fee {
    /// Create a new Fee with inner randomness
    #[must_use]
    pub fn new<R: RngCore + CryptoRng>(
        rng: &mut R,
        pk: &PublicKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        let r = JubJubScalar::random(&mut *rng);

        let sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];

        Self::deterministic(&r, pk, gas_limit, gas_price, &sender_blinder)
    }

    /// Create a new Fee without inner randomness
    #[must_use]
    pub fn deterministic(
        r: &JubJubScalar,
        pk: &PublicKey,
        gas_limit: u64,
        gas_price: u64,
        sender_blinder: &[JubJubScalar; 2],
    ) -> Self {
        let stealth_address = pk.gen_stealth_address(r);
        let sender =
            Sender::encrypt(stealth_address.note_pk(), pk, sender_blinder);

        Fee {
            gas_limit,
            gas_price,
            stealth_address,
            sender,
        }
    }

    /// Calculate the max-fee.
    #[must_use]
    pub fn max_fee(&self) -> u64 {
        self.gas_limit * self.gas_price
    }

    /// Return a hash represented by `H(gas_limit, gas_price, H([note_pk]))`
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        let npk = self.stealth_address.note_pk().as_ref().to_hash_inputs();

        let hash_inputs = [
            BlsScalar::from(self.gas_limit),
            BlsScalar::from(self.gas_price),
            npk[0],
            npk[1],
        ];
        Hash::digest(Domain::Other, &hash_inputs)[0]
    }

    /// Generates a remainder from the fee and the given gas consumed
    #[must_use]
    pub fn gen_remainder_note(&self, gas_consumed: u64) -> Note {
        // Consuming more gas than the limit provided should never
        // occur, and it's not responsability of the `Fee` to check that.
        // Here defensively ensure it's not panicking, capping the gas
        // consumed to the gas limit.
        let gas_consumed = cmp::min(gas_consumed, self.gas_limit);
        let gas_changes = (self.gas_limit - gas_consumed) * self.gas_price;

        Note::transparent_stealth(
            self.stealth_address,
            gas_changes,
            self.sender,
        )
    }
}

const SIZE: usize = 2 * u64::SIZE + StealthAddress::SIZE + Sender::SIZE;

impl Serializable<SIZE> for Fee {
    type Error = BytesError;

    /// Converts a Fee into it's byte representation
    #[must_use]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[..u64::SIZE].copy_from_slice(&self.gas_limit.to_bytes());
        let mut start = u64::SIZE;
        buf[start..start + u64::SIZE]
            .copy_from_slice(&self.gas_price.to_bytes());
        start += u64::SIZE;
        buf[start..start + StealthAddress::SIZE]
            .copy_from_slice(&self.stealth_address.to_bytes());
        start += StealthAddress::SIZE;
        buf[start..start + Sender::SIZE]
            .copy_from_slice(&self.sender.to_bytes());

        buf
    }

    /// Attempts to convert a byte representation of a fee into a `Fee`,
    /// failing if the input is invalid
    fn from_bytes(bytes: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut reader = &bytes[..];

        let gas_limit = u64::from_reader(&mut reader)?;
        let gas_price = u64::from_reader(&mut reader)?;
        let stealth_address = StealthAddress::from_reader(&mut reader)?;
        let sender = Sender::from_reader(&mut reader)?;

        Ok(Fee {
            gas_limit,
            gas_price,
            stealth_address,
            sender,
        })
    }
}

/// Additional data used to identify the origin of a [`Note`] when the
/// [`Sender`] is a `Contract`.
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SenderAccount {
    /// The unique identifier of a contract.
    pub contract: ContractId,
    /// The unique identifier of the account on that contract.
    pub account: BlsPublicKey,
}

impl From<SenderAccount> for Sender {
    fn from(sender: SenderAccount) -> Self {
        let mut contract_info = [0u8; 4 * JubJubAffine::SIZE];
        contract_info[0..32].copy_from_slice(&sender.contract[..]);
        contract_info[32..].copy_from_slice(&sender.account.to_bytes());

        Sender::ContractInfo(contract_info)
    }
}
