// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_schnorr::Signature;
use dusk_jubjub::JubJubAffine;
use rkyv::{Archive, Deserialize, Serialize};

/// SP Public Key.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SPPublicKey {
    pub sp_pk: JubJubAffine,
}

/// User Public Key.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UserPublicKey {
    pub user_pk: JubJubAffine,
}

/// License Nullifier.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseNullifier {
    pub value: BlsScalar,
}

/// License Request.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseRequest {
    pub sp_public_key: SPPublicKey,
}

/// License Session.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseSession {
    pub nullifier: LicenseNullifier,
}

/// License.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct License {
    pub user_pk: UserPublicKey,
    pub sp_pk: SPPublicKey,
    pub sig_lic: Signature,
}
