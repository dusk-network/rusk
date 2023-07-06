// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(dead_code)]

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubExtended, JubJubScalar};
use dusk_pki::PublicSpendKey;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

pub(crate) enum Query {}

impl Query {
    pub const HASH: &str = "hash";
    pub const POSEIDON_HASH: &str = "poseidon_hash";
    pub const VERIFY_PROOF: &str = "verify_proof";
    pub const VERIFY_SCHNORR: &str = "verify_schnorr";
    pub const VERIFY_BLS: &str = "verify_bls";
}

pub(crate) enum Metadata {}

impl Metadata {
    pub const BLOCK_HEIGHT: &str = "block_height";
}

/// Enum representing all possible payment configurations.
#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
#[repr(C)]
pub enum PaymentInfo {
    /// Only transparent notes are accepted.
    Transparent(Option<PublicSpendKey>),
    /// Only obfuscated notes are accepted.
    Obfuscated(Option<PublicSpendKey>),
    /// Any type of note is accepted.
    Any(Option<PublicSpendKey>),
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
