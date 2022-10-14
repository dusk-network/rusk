// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use hex::ToHex;
use rand::rngs::StdRng;
use rand_core::SeedableRng;
use std::cmp::Ordering;

pub const PUBLIC_BLS_SIZE: usize = 96;

/// PublicKey is a thin wrapper of dusk_bls12_381_sign::PublicKey
#[derive(Eq, PartialEq, Clone, Copy, Default)]
pub struct PublicKey(dusk_bls12_381_sign::PublicKey);

impl PublicKey {
    pub fn new(pk: dusk_bls12_381_sign::PublicKey) -> Self {
        Self(pk)
    }

    /// from_sk_seed_u64 generates a sk from the specified seed and returns the associated public key
    pub fn from_sk_seed_u64(state: u64) -> Self {
        let rng = &mut StdRng::seed_from_u64(state);
        let sk = dusk_bls12_381_sign::SecretKey::random(rng);
        Self(dusk_bls12_381_sign::PublicKey::from(&sk))
    }

    pub fn to_raw_bytes(&self) -> [u8; 193] {
        self.0.to_raw_bytes()
    }

    pub fn to_bls_pk(&self) -> dusk_bls12_381_sign::PublicKey {
        self.0
    }

    pub fn encode_short_hex(&self) -> String {
        let mut hex = self.0.to_bytes().encode_hex::<String>();
        hex.truncate(16);
        hex
    }
}

impl PartialOrd<PublicKey> for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0
            .pk_t()
            .to_bytes()
            .partial_cmp(&other.0.pk_t().to_bytes())
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.pk_t().to_bytes().cmp(&other.0.pk_t().to_bytes())
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            PublicKey(ref v) => {
                let mut hex = v.to_bytes().encode_hex::<String>();
                hex.truncate(16);

                let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "PublicKey");
                let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &hex);
                ::core::fmt::DebugTuple::finish(debug_trait_builder)
            }
        }
    }
}
