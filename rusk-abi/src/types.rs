// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(dead_code)]

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubExtended, JubJubScalar};
use phoenix_core::PublicKey;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

pub(crate) enum Query {}

impl Query {
    pub const HASH: &'static str = "hash";
    pub const POSEIDON_HASH: &'static str = "poseidon_hash";
    pub const VERIFY_PROOF: &'static str = "verify_proof";
    pub const VERIFY_SCHNORR: &'static str = "verify_schnorr";
    pub const VERIFY_BLS: &'static str = "verify_bls";
}

pub(crate) enum Metadata {}

impl Metadata {
    pub const BLOCK_HEIGHT: &'static str = "block_height";
}

/// Enum representing all possible payment configurations.
#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
#[repr(C)]
pub enum PaymentInfo {
    /// Only transparent notes are accepted.
    Transparent(Option<PublicKey>),
    /// Only obfuscated notes are accepted.
    Obfuscated(Option<PublicKey>),
    /// Any type of note is accepted.
    Any(Option<PublicKey>),
}

/// Enum that represents all possible types of public inputs
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum PublicInput {
    /// A Public Input Point
    Point(JubJubAffine),
    /// A Public Input BLS Scalar
    BlsScalar(BlsScalar),
    /// A Public Input JubJub Scalar
    JubJubScalar(JubJubScalar),
}

impl PublicInput {
    /// Update the [`blake2b_simd::State`] with the public input.
    pub(crate) fn update_hasher(&self, hasher: &mut blake2b_simd::State) {
        match self {
            Self::Point(p) => {
                let u = p.get_u();
                let v = p.get_v();

                hasher.update(cast_to_bytes(&u.0));
                hasher.update(cast_to_bytes(&v.0));
            }
            Self::BlsScalar(s) => {
                hasher.update(cast_to_bytes(&s.0));
            }
            Self::JubJubScalar(s) => {
                let bytes = s.to_bytes();
                hasher.update(&bytes);
            }
        }
    }
}

fn cast_to_bytes(bytes: &[u64; 4]) -> &[u8; 32] {
    // SAFETY: The size of the array is the same, so this is safe.
    unsafe { &*(bytes as *const [u64; 4] as *const [u8; 32]) }
}

impl From<BlsScalar> for PublicInput {
    fn from(s: BlsScalar) -> PublicInput {
        Self::BlsScalar(s)
    }
}

impl From<u64> for PublicInput {
    fn from(n: u64) -> PublicInput {
        Self::BlsScalar(n.into())
    }
}

impl From<JubJubScalar> for PublicInput {
    fn from(s: JubJubScalar) -> PublicInput {
        Self::JubJubScalar(s)
    }
}

impl From<JubJubAffine> for PublicInput {
    fn from(p: JubJubAffine) -> PublicInput {
        Self::Point(p)
    }
}

impl From<JubJubExtended> for PublicInput {
    fn from(p: JubJubExtended) -> PublicInput {
        JubJubAffine::from(p).into()
    }
}

impl<T> From<&T> for PublicInput
where
    T: Clone + Into<PublicInput>,
{
    fn from(t: &T) -> PublicInput {
        t.clone().into()
    }
}
