// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod header;
pub use header::{Header, Seed};

mod block;
pub use block::{Block, BlockWithLabel, Hash, Label};

use crate::bls::{self, PublicKeyBytes};
use crate::message::payload::{RatificationResult, Vote};
use crate::Serializable;

use dusk_bytes::DeserializableSlice;
use rusk_abi::hash::Hasher;
use sha3::Digest;
use std::io::{self, Read, Write};

use execution_core::{
    transfer::Transaction as PhoenixTransaction, BlsPublicKey,
};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u32,
    pub r#type: u32,
    pub inner: PhoenixTransaction,
}

impl From<PhoenixTransaction> for Transaction {
    fn from(value: PhoenixTransaction) -> Self {
        Self {
            inner: value,
            r#type: 1,
            version: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpentTransaction {
    pub inner: Transaction,
    pub block_height: u64,
    pub gas_spent: u64,
    pub err: Option<String>,
}

impl Transaction {
    /// Computes the hash of the transaction.
    ///
    /// This method returns the [hash](rusk_abi::hash()) of the entire
    /// transaction in its serialized form
    ///
    /// ### Returns
    /// An array of 32 bytes representing the hash of the transaction.
    pub fn hash(&self) -> [u8; 32] {
        Hasher::digest(self.inner.to_var_bytes()).to_bytes()
    }

    /// Computes the transaction ID.
    ///
    /// The transaction ID is a unique identifier for the transaction.
    /// Unlike the [`hash()`](#method.hash) method, which is computed over the
    /// entire transaction, the transaction ID is derived from specific
    /// fields of the transaction and serves as a unique identifier of the
    /// transaction itself.
    ///
    /// ### Returns
    /// An array of 32 bytes representing the transaction ID.
    pub fn id(&self) -> [u8; 32] {
        Hasher::digest(self.inner.to_hash_input_bytes()).to_bytes()
    }

    pub fn gas_price(&self) -> u64 {
        self.inner.payload().fee().gas_price
    }
    pub fn to_nullifiers(&self) -> Vec<[u8; 32]> {
        self.inner
            .payload()
            .tx_skeleton()
            .nullifiers()
            .iter()
            .map(|n| n.to_bytes())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct Attestation {
    pub result: RatificationResult,
    pub validation: StepVotes,
    pub ratification: StepVotes,
}

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct StepVotes {
    pub bitset: u64,
    pub(crate) aggregate_signature: Signature,
}

impl StepVotes {
    pub fn new(aggregate_signature: [u8; 48], bitset: u64) -> StepVotes {
        StepVotes {
            bitset,
            aggregate_signature: Signature(aggregate_signature),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bitset == 0 || self.aggregate_signature.is_zeroed()
    }

    pub fn aggregate_signature(&self) -> &Signature {
        &self.aggregate_signature
    }
}

/// a wrapper of 48-sized array to facilitate Signature
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Signature([u8; 48]);

impl Signature {
    pub const EMPTY: [u8; 48] = [0u8; 48];

    fn is_zeroed(&self) -> bool {
        self.0 == Self::EMPTY
    }
    pub fn inner(&self) -> &[u8; 48] {
        &self.0
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature")
            .field("signature", &to_str(&self.0))
            .finish()
    }
}

impl From<[u8; 48]> for Signature {
    fn from(value: [u8; 48]) -> Self {
        Self(value)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self(Self::EMPTY)
    }
}

impl PartialEq<Self> for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type
            && self.version == other.version
            && self.id() == other.id()
    }
}

impl Eq for Transaction {}

impl PartialEq<Self> for SpentTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.gas_spent == other.gas_spent
    }
}

impl Eq for SpentTransaction {}

/// Includes a failed attestation and the key of the expected block
/// generator
pub type IterationInfo = (Attestation, PublicKeyBytes);

/// Defines a set of attestations of any former iterations
#[derive(Default, Eq, PartialEq, Clone)]
pub struct IterationsInfo {
    /// Represents a list of attestations where position is the iteration
    /// number
    pub att_list: Vec<Option<IterationInfo>>,
}

impl IterationsInfo {
    pub fn new(attestations: Vec<Option<IterationInfo>>) -> Self {
        Self {
            att_list: attestations,
        }
    }

    pub fn to_missed_generators(&self) -> Result<Vec<BlsPublicKey>, io::Error> {
        self.to_missed_generators_bytes()
        .map(|pk| BlsPublicKey::from_slice(pk.inner()).map_err(|e|{
            tracing::error!("Unable to generate missing generators from failed_iterations: {e:?}");
            io::Error::new(io::ErrorKind::InvalidData, "Error in deserialize")
        }))
        .collect()
    }

    pub fn to_missed_generators_bytes(
        &self,
    ) -> impl Iterator<Item = &PublicKeyBytes> {
        self.att_list
            .iter()
            .flatten()
            .filter(|(c, _)| {
                c.result == RatificationResult::Fail(Vote::NoCandidate)
            })
            .map(|(_, pk)| pk)
    }
}

/// Encode a byte array into a shortened HEX representation.
pub fn to_str<const N: usize>(bytes: &[u8; N]) -> String {
    let e = hex::encode(bytes);
    if e.len() != bytes.len() * 2 {
        return String::from("invalid hex");
    }

    const OFFSET: usize = 16;
    let (first, last) = e.split_at(OFFSET);
    let (_, second) = last.split_at(e.len() - 2 * OFFSET);

    first.to_owned() + "..." + second
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use super::*;
    use crate::bls::PublicKeyBytes;
    use execution_core::transfer::{ContractCall, Fee, Payload};
    use execution_core::{
        BlsScalar, JubJubScalar, Note, PublicKey, SecretKey, TxSkeleton,
    };
    use rand::Rng;

    impl<T> Dummy<T> for Block {
        /// Creates a block with 3 transactions and random header.
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let txs = vec![
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
            ];
            let header: Header = Faker.fake();

            Block::new(header, txs).expect("valid hash")
        }
    }

    impl<T> Dummy<T> for Transaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            gen_dummy_tx(1_000_000)
        }
    }

    impl<T> Dummy<T> for SpentTransaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            let tx = gen_dummy_tx(1_000_000);
            SpentTransaction {
                inner: tx,
                block_height: 0,
                gas_spent: 3,
                err: Some("error".to_string()),
            }
        }
    }

    impl<T> Dummy<T> for PublicKeyBytes {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let rand_val = rng.gen::<[u8; 32]>();
            let mut bls_key = [0u8; 96];
            bls_key[..32].copy_from_slice(&rand_val);
            bls::PublicKeyBytes(bls_key)
        }
    }

    impl<T> Dummy<T> for bls::PublicKey {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let rand_val = rng.gen();
            bls::PublicKey::from_sk_seed_u64(rand_val)
        }
    }

    impl<T> Dummy<T> for Signature {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let rand_val = rng.gen::<[u8; 32]>();
            let mut rand_signature = Self::EMPTY;
            rand_signature[..32].copy_from_slice(&rand_val);

            Signature(rand_signature)
        }
    }

    impl<T> Dummy<T> for IterationsInfo {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let att_list = vec![
                None,
                Some(Faker.fake_with_rng(rng)),
                None,
                Some(Faker.fake_with_rng(rng)),
                None,
            ];
            IterationsInfo { att_list }
        }
    }

    /// Generates a decodable transaction from a fixed blob with a specified
    /// gas price.
    pub fn gen_dummy_tx(gas_price: u64) -> Transaction {
        let pk = PublicKey::from(&SecretKey::new(
            JubJubScalar::from(42u64),
            JubJubScalar::from(42u64),
        ));
        let gas_limit = 1;

        let fee = Fee::deterministic(
            &JubJubScalar::from(5u64),
            &pk,
            gas_limit,
            gas_price,
            &[JubJubScalar::from(9u64), JubJubScalar::from(10u64)],
        );

        let tx_skeleton = TxSkeleton {
            root: BlsScalar::from(12345u64),
            nullifiers: vec![
                BlsScalar::from(1u64),
                BlsScalar::from(2u64),
                BlsScalar::from(3u64),
            ],
            outputs: [Note::empty(), Note::empty()],
            max_fee: gas_price * gas_limit,
            deposit: 0,
        };

        let contract_call =
            ContractCall::new([21; 32], "some_method", &()).unwrap();

        let payload =
            Payload::new(tx_skeleton, fee, false, Some(contract_call));
        let proof = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        PhoenixTransaction::new(payload, proof).into()
    }
}
