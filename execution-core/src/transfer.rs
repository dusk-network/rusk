// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to Dusk's transfer contract that are shared across the
//! network.

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
    BlsPublicKey, BlsScalar, BlsSecretKey, BlsSignature, JubJubScalar, Note,
    PublicKey, SchnorrSecretKey, SchnorrSignature, Sender, StealthAddress,
};

mod transaction;
pub use transaction::{
    MoonlightPayload, MoonlightTransaction, PhoenixPayload, PhoenixTransaction,
    Transaction,
};

use crate::bytecode::Bytecode;
use crate::reader::{read_arr, read_str, read_vec};

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

/// A Moonlight account's information.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct AccountData {
    /// Number used for replay protection.
    pub nonce: u64,
    /// Account balance.
    pub balance: u64,
}

/// Withdrawal information, proving the intent of a user to withdraw from a
/// contract.
///
/// This structure is meant to be passed to a contract by a caller. The contract
/// is then responsible for calling `withdraw` in the transfer contract to
/// settle it, if it wants to allow the withdrawal.
///
/// e.g. the stake contract uses it as a call argument for the `unstake`
/// function
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    contract: ContractId,
    value: u64,
    receiver: WithdrawReceiver,
    token: WithdrawReplayToken,
    signature: WithdrawSignature,
}

impl Withdraw {
    /// Create a new contract withdrawal.
    ///
    /// # Panics
    /// When the receiver does not match the secret key passed.
    #[must_use]
    pub fn new<'a, R: RngCore + CryptoRng>(
        rng: &mut R,
        sk: impl Into<WithdrawSecretKey<'a>>,
        contract: ContractId,
        value: u64,
        receiver: WithdrawReceiver,
        token: WithdrawReplayToken,
    ) -> Self {
        let mut withdraw = Self {
            contract,
            value,
            receiver,
            token,
            signature: WithdrawSignature::Moonlight(BlsSignature::default()),
        };

        let sk = sk.into();

        match (&sk, &receiver) {
            (WithdrawSecretKey::Phoenix(_), WithdrawReceiver::Moonlight(_)) => {
                panic!("Moonlight receiver with phoenix signer");
            }
            (WithdrawSecretKey::Moonlight(_), WithdrawReceiver::Phoenix(_)) => {
                panic!("Phoenix receiver with moonlight signer");
            }
            _ => {}
        }

        let msg = withdraw.signature_message();

        match sk {
            WithdrawSecretKey::Phoenix(sk) => {
                let digest = BlsScalar::hash_to_scalar(&msg);
                let signature = sk.sign(rng, digest);
                withdraw.signature = signature.into();
            }
            WithdrawSecretKey::Moonlight(sk) => {
                let pk = BlsPublicKey::from(sk);
                let signature = sk.sign(&pk, &msg);
                withdraw.signature = signature.into();
            }
        }

        withdraw
    }

    /// The contract to withraw from.
    #[must_use]
    pub fn contract(&self) -> &ContractId {
        &self.contract
    }

    /// The amount to withdraw.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.value
    }

    /// The receiver of the value.
    #[must_use]
    pub fn receiver(&self) -> &WithdrawReceiver {
        &self.receiver
    }

    /// The unique token to prevent replay.
    #[must_use]
    pub fn token(&self) -> &WithdrawReplayToken {
        &self.token
    }

    /// Signature of the withdrawal.
    #[must_use]
    pub fn signature(&self) -> &WithdrawSignature {
        &self.signature
    }

    /// Return the message that is used as the input to the signature.
    ///
    /// This message is *not* the one that is meant to be signed on making a
    /// withdrawal. Instead it is meant to be used by structures wrapping
    /// withdrawals to offer additional functionality.
    ///
    /// To see the signature used to sign a withdrawal, see
    /// [`WithdrawPayload::signature_message`].
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.contract);
        bytes.extend(self.value.to_bytes());

        match self.receiver {
            WithdrawReceiver::Phoenix(address) => {
                bytes.extend(address.to_bytes());
            }
            WithdrawReceiver::Moonlight(account) => {
                bytes.extend(account.to_bytes());
            }
        }

        match &self.token {
            WithdrawReplayToken::Phoenix(nullifiers) => {
                for n in nullifiers {
                    bytes.extend(n.to_bytes());
                }
            }
            WithdrawReplayToken::Moonlight(nonce) => {
                bytes.extend(nonce.to_bytes());
            }
        }

        bytes
    }

    /// Returns the message that should be "mixed in" as input for a signature
    /// of an item that wraps a [`Withdraw`].
    ///
    /// One example of this is [`stake::Withdraw`].
    #[must_use]
    pub fn wrapped_signature_message(&self) -> Vec<u8> {
        let mut bytes = self.signature_message();
        bytes.extend(self.signature.to_var_bytes());
        bytes
    }
}

/// The receiver of the [`Withdraw`] value.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawReceiver {
    /// The stealth address to withdraw to, when the withdrawal is into Phoenix
    /// notes.
    Phoenix(StealthAddress),
    /// The account to withdraw to, when the withdrawal is to a Moonlight
    /// account.
    Moonlight(BlsPublicKey),
}

/// The token used for replay protection in a [`Withdraw`]. This is the same as
/// the encapsulating transaction's fields.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawReplayToken {
    /// The nullifiers of the encapsulating Phoenix transaction, when the
    /// transaction is paid for using Phoenix notes.
    Phoenix(Vec<BlsScalar>),
    /// The nonce of the encapsulating Moonlight transaction, when the
    /// transaction is paid for using a Moonlight account.
    Moonlight(u64),
}

/// The secret key used for signing a [`Withdraw`].
///
/// When the withdrawal is into Phoenix notes, a [`SchnorrSecretKey`] should be
/// used. When the withdrawal is into a Moonlight account an
/// [`BlsSecretKey`] should be used.
#[derive(Debug, Clone, PartialEq)]
pub enum WithdrawSecretKey<'a> {
    /// The secret key used to sign a withdrawal into Phoenix notes.
    Phoenix(&'a SchnorrSecretKey),
    /// The secret key used to sign a withdrawal into a Moonlight account.
    Moonlight(&'a BlsSecretKey),
}

impl<'a> From<&'a SchnorrSecretKey> for WithdrawSecretKey<'a> {
    fn from(sk: &'a SchnorrSecretKey) -> Self {
        Self::Phoenix(sk)
    }
}

impl<'a> From<&'a BlsSecretKey> for WithdrawSecretKey<'a> {
    fn from(sk: &'a BlsSecretKey) -> Self {
        Self::Moonlight(sk)
    }
}

/// The signature used for a [`Withdraw`].
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawSignature {
    /// A transaction withdrawing to Phoenix must sign using their
    /// [`SecretKey`].
    Phoenix(SchnorrSignature),
    /// A transaction withdrawing to Moonlight - must sign using their
    /// [`BlsSecretKey`].
    Moonlight(BlsSignature),
}

impl WithdrawSignature {
    fn to_var_bytes(&self) -> Vec<u8> {
        match self {
            WithdrawSignature::Phoenix(sig) => sig.to_bytes().to_vec(),
            WithdrawSignature::Moonlight(sig) => sig.to_bytes().to_vec(),
        }
    }
}

impl From<SchnorrSignature> for WithdrawSignature {
    fn from(sig: SchnorrSignature) -> Self {
        Self::Phoenix(sig)
    }
}

impl From<BlsSignature> for WithdrawSignature {
    fn from(sig: BlsSignature) -> Self {
        Self::Moonlight(sig)
    }
}

/// Data for either contract call or contract deployment.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum ContractExec {
    /// Data for a contract call.
    Call(ContractCall),
    /// Data for a contract deployment.
    Deploy(ContractDeploy),
}

impl From<ContractCall> for ContractExec {
    fn from(c: ContractCall) -> Self {
        ContractExec::Call(c)
    }
}

impl From<ContractDeploy> for ContractExec {
    fn from(d: ContractDeploy) -> Self {
        ContractExec::Deploy(d)
    }
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
    /// Errors when the bytes are not canonical.
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

    /// Generates a remainder from the fee and the given gas consumed.
    ///
    /// If there is a deposit, it means that the deposit hasn't been picked up
    /// by the contract. In this case, it is added to the remainder note.
    #[must_use]
    pub fn gen_remainder_note(
        &self,
        gas_consumed: u64,
        deposit: Option<u64>,
    ) -> Note {
        // Consuming more gas than the limit provided should never occur, and
        // it's not the responsibility of the `Fee` to check that.
        // Here defensively ensure it's not panicking, capping the gas consumed
        // to the gas limit.
        let gas_consumed = cmp::min(gas_consumed, self.gas_limit);
        let gas_changes = (self.gas_limit - gas_consumed) * self.gas_price;

        Note::transparent_stealth(
            self.stealth_address,
            gas_changes + deposit.unwrap_or_default(),
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
