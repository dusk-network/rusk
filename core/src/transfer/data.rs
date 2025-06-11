// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Extra data that may be sent with the `data` field of either transaction
//! type.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use piecrust_uplink::StandardBufSerializer;
use rkyv::ser::serializers::{
    BufferScratch, BufferSerializer, CompositeSerializer,
};
use rkyv::ser::Serializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use c_kzg::{
    Blob as KzgBlob, KzgCommitment, KzgProof, BYTES_PER_BLOB,
    BYTES_PER_COMMITMENT, BYTES_PER_PROOF,
};

use crate::abi::ContractId;
use crate::Error;

/// The maximum size of a memo.
pub const MAX_MEMO_SIZE: usize = 512;

/// Data for either contract call or contract deployment.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[allow(clippy::large_enum_variant)]
pub enum TransactionData {
    /// Data for a contract call.
    Call(ContractCall),
    /// Data for a contract deployment.
    Deploy(ContractDeploy),
    /// Additional data added to a transaction, that is not a deployment or a
    /// call.
    Memo(Vec<u8>),
    /// Data for BlobTx
    /// BlobHashes are the hashes of the blobs that are included in the
    /// transaction.
    ///
    /// NOTE: The type `BlobHashes` is implemented as `Vec<u8>`, the same as in
    /// `Memo(Vec<u8>)`. This ensures data compatibility, so that when
    /// reconstructing or repacking a transaction, the data contained in
    /// `BlobHashes` can be moved to `Memo(Vec<u8>)` without loss or
    /// conversion. Changing the `TransactionData` type in this way only
    /// affects the client side and consensus layer, but does NOT affect
    /// already compiled and deployed system smart contracts.
    //Option<BlobSidecar> stripped before minting to block
    Blob(BlobHashes, Option<BlobSidecar>),
}

// BlobTx represents an EIP-4844 transaction.
// type BlobTx struct {
//     ChainID    *uint256.Int
//     Nonce      uint64
//     GasTipCap  *uint256.Int // a.k.a. maxPriorityFeePerGas
//     GasFeeCap  *uint256.Int // a.k.a. maxFeePerGas
//     Gas        uint64
//     To         common.Address
//     Value      *uint256.Int
//     Data       []byte
//     AccessList AccessList
//     BlobFeeCap *uint256.Int // a.k.a. maxFeePerBlobGas
//     BlobHashes []common.Hash

//     // A blob transaction can optionally contain blobs. This field must be
// set when BlobTx     // is used to create a transaction for signing.
//     Sidecar *BlobTxSidecar `rlp:"-"`

//     // Signature values
//     V *uint256.Int
//     R *uint256.Int
//     S *uint256.Int
// }
// type BlobTxSidecar struct {
//     Blobs       []kzg4844.Blob       // Blobs needed by the blob pool
//     Commitments []kzg4844.Commitment // Commitments needed by the blob pool
//     Proofs      []kzg4844.Proof      // Proofs needed by the blob pool
// }

/// A type alias for a vector of blob hashes.
pub type BlobHashes = Vec<u8>;

/// Represents a KZG blob (EIP-4844).
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[repr(transparent)]
pub struct Blob(pub [u8; BYTES_PER_BLOB]);

/// Blob ⇄ KzgBlob
impl From<Blob> for KzgBlob {
    fn from(blob: Blob) -> Self {
        KzgBlob::from_bytes(&blob.0).expect("Invalid blob bytes")
    }
}

impl From<KzgBlob> for Blob {
    fn from(blob: KzgBlob) -> Self {
        Blob(blob.as_ref().try_into().expect("Invalid blob length"))
    }
}

/// Represents a KZG commitment.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
#[repr(transparent)]
pub struct Commitment(pub [u8; BYTES_PER_COMMITMENT]);

/// Commitment ⇄ KzgCommitment
impl From<Commitment> for KzgCommitment {
    fn from(commitment: Commitment) -> Self {
        KzgCommitment::from_bytes(&commitment.0)
            .expect("Invalid commitment bytes")
    }
}

impl From<KzgCommitment> for Commitment {
    fn from(commitment: KzgCommitment) -> Self {
        Commitment(
            commitment
                .as_ref()
                .try_into()
                .expect("Invalid commitment length"),
        )
    }
}

/// Represents a KZG proof.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
#[repr(transparent)]
pub struct Proof(pub [u8; BYTES_PER_PROOF]);

/// Proof ⇄ KzgProof
impl From<Proof> for KzgProof {
    fn from(proof: Proof) -> Self {
        KzgProof::from_bytes(&proof.0).expect("Invalid proof bytes")
    }
}

impl From<KzgProof> for Proof {
    fn from(proof: KzgProof) -> Self {
        Proof(proof.as_ref().try_into().expect("Invalid proof length"))
    }
}

/// A sidecar for blobs, commitments, and proofs used in the blob pool.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct BlobSidecar {
    /// Blobs needed by the blob pool.
    /// Blob is a wrapper around the c_kzg::Blob type.
    pub blobs: Vec<Blob>,
    /// Commitments needed by the blob pool.
    /// Commitment is a wrapper around the c_kzg::Commitment type.
    pub commitments: Vec<Commitment>,
    /// Proofs needed by the blob pool.
    /// Proof is a wrapper around the c_kzg::Proof type.
    pub proofs: Vec<Proof>,
}

// impl BlobSidecar {
//     todo!("Implement BlobSidecar methods");
// }

impl From<ContractCall> for TransactionData {
    fn from(c: ContractCall) -> Self {
        TransactionData::Call(c)
    }
}

impl From<ContractDeploy> for TransactionData {
    fn from(d: ContractDeploy) -> Self {
        TransactionData::Deploy(d)
    }
}

impl From<Vec<u8>> for TransactionData {
    fn from(d: Vec<u8>) -> Self {
        TransactionData::Memo(d)
    }
}

impl From<String> for TransactionData {
    fn from(d: String) -> Self {
        TransactionData::Memo(d.as_bytes().to_vec())
    }
}

/// Data for performing a contract deployment
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractDeploy {
    /// Bytecode of the contract to be deployed.
    pub bytecode: ContractBytecode,
    /// Owner of the contract to be deployed.
    pub owner: Vec<u8>,
    /// Init method arguments of the deployed contract.
    pub init_args: Option<Vec<u8>>,
    /// Nonce for contract id uniqueness and vanity
    pub nonce: u64,
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

impl ContractDeploy {
    /// Serialize a `ContractDeploy` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(&self.bytecode.to_var_bytes());

        bytes.extend((self.owner.len() as u64).to_bytes());
        bytes.extend(&self.owner);

        match &self.init_args {
            Some(init_args) => {
                bytes.push(1);
                bytes.extend((init_args.len() as u64).to_bytes());
                bytes.extend(init_args);
            }
            None => bytes.push(0),
        }

        bytes.extend(self.nonce.to_bytes());

        bytes
    }

    /// Deserialize a `ContractDeploy` from a byte buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let bytecode = ContractBytecode::from_buf(&mut buf)?;

        let owner = crate::read_vec(&mut buf)?;

        let init_args = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(crate::read_vec(&mut buf)?),
            _ => return Err(BytesError::InvalidData),
        };

        let nonce = u64::from_reader(&mut buf)?;

        Ok(Self {
            bytecode,
            owner,
            init_args,
            nonce,
        })
    }
}

impl ContractCall {
    /// Creates a new contract call with empty `fn_args`.
    ///
    /// Initializes a contract call by setting the function arguments to an
    /// empty vector.
    ///
    /// # Parameters
    /// - `contract`: A value convertible into a `ContractId`, representing the
    ///   target contract.
    /// - `fn_name`: A value convertible into a `String`, specifying the name of
    ///   the function to be called.
    pub fn new(
        contract: impl Into<ContractId>,
        fn_name: impl Into<String>,
    ) -> Self {
        Self {
            contract: contract.into(),
            fn_name: fn_name.into(),
            fn_args: vec![],
        }
    }

    /// Consumes `self` and returns a new contract call with raw function
    /// arguments.
    ///
    /// Updates the contract call with raw serialized arguments provided as a
    /// `Vec<u8>`.
    ///
    /// # Parameters
    /// - `fn_args`: A `Vec<u8>` representing pre-serialized function arguments.
    #[must_use]
    pub fn with_raw_args(mut self, fn_args: Vec<u8>) -> Self {
        self.fn_args = fn_args;
        self
    }

    /// Consumes `self` and returns a new contract call with serialized function
    /// arguments.
    ///
    /// Serializes the provided function arguments using `rkyv` serialization
    /// and returns an updated contract call.
    ///
    /// # Parameters
    /// - `fn_args`: A reference to an object implementing `Serialize` for the
    ///   given `AllocSerializer`.
    ///
    /// # Returns
    /// - `Ok(Self)`: If the serialization is successful.
    /// - `Err(Error::Rkyv)`: If the `rkyv` serialization fails.
    ///
    /// # Errors
    /// Returns an error if `rkyv` serialization fails.
    pub fn with_args<A>(self, fn_arg: &A) -> Result<Self, Error>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        // scratch-space and page-size values taken from piecrust-uplink
        const SCRATCH_SPACE: usize = 1024;
        const PAGE_SIZE: usize = 0x1000;

        let mut sbuf = [0u8; SCRATCH_SPACE];
        let scratch = BufferScratch::new(&mut sbuf);
        let mut buffer = [0u8; PAGE_SIZE];
        let ser = BufferSerializer::new(&mut buffer[..]);
        let mut ser = CompositeSerializer::new(ser, scratch, Infallible);

        ser.serialize_value(fn_arg)
            .map_err(|e| Error::Rkyv(format!("{e:?}")))?;
        let pos = ser.pos();

        let fn_args = buffer[..pos].to_vec();

        Ok(self.with_raw_args(fn_args))
    }

    /// Serialize a `ContractCall` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.contract.as_bytes());

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

        let contract = crate::read_arr::<32>(&mut buf)?;

        let fn_name = crate::read_str(&mut buf)?;

        let fn_args = crate::read_vec(&mut buf)?;

        Ok(Self {
            contract: contract.into(),
            fn_name,
            fn_args,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
/// Holds bytes of bytecode and its hash.
pub struct ContractBytecode {
    /// Blake3 hash of the bytecode bytes.
    pub hash: [u8; 32],
    /// Bytecode bytes.
    pub bytes: Vec<u8>,
}

impl ContractBytecode {
    /// Provides contribution bytes for an external hash.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        self.hash.to_vec()
    }

    /// Serializes this object into a variable length buffer
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.hash);
        bytes.extend((self.bytes.len() as u64).to_bytes());
        bytes.extend(&self.bytes);
        bytes
    }

    /// Deserialize from a bytes buffer.
    /// Resets buffer to a position after the bytes read.
    ///
    /// # Errors
    /// Errors when the bytes are not available.
    pub fn from_buf(buf: &mut &[u8]) -> Result<Self, BytesError> {
        let hash = crate::read_arr::<32>(buf)?;
        let bytes = crate::read_vec(buf)?;
        Ok(Self { hash, bytes })
    }
}
