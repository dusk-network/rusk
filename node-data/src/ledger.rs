// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod header;
pub use header::{Header, Seed};

mod block;
pub use block::{Block, BlockWithLabel, Hash, Label};

mod transaction;
pub use transaction::{SpentTransaction, Transaction};

use crate::bls::{self, PublicKeyBytes};
use crate::message::payload::{RatificationResult, Vote};
use crate::Serializable;

use dusk_bytes::DeserializableSlice;
use rusk_abi::hash::Hasher;
use sha3::Digest;
use std::io::{self, Read, Write};

use execution_core::BlsPublicKey;

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

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
    use rand::Rng;
    use transaction::faker::gen_dummy_tx;

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
}
