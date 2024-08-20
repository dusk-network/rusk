// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::Serialize;

use super::*;

use crate::message::payload::RatificationResult;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct Attestation {
    pub result: RatificationResult,
    pub validation: StepVotes,
    pub ratification: StepVotes,
}

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
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

/// A wrapper of 48-sized array to facilitate Signature
#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub struct Signature(
    #[serde(serialize_with = "crate::serialize_hex")] [u8; 48],
);

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

/// Defines a set of attestations of former iterations
#[derive(Default, Eq, PartialEq, Clone, Serialize)]
#[serde(transparent)]
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
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use super::*;
    use crate::bls;
    use rand::Rng;

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
