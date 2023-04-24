// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use core::cmp::Ordering;
use rkyv::{Archive, Deserialize, Serialize};

/// SP Public Key.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialOrd,
    Eq,
    PartialEq,
    Archive,
    Serialize,
    Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct SPPublicKey {
    pub sp_pk: u64,
}

impl Ord for SPPublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sp_pk.cmp(&other.sp_pk)
    }
}

/// User Public Key.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialOrd,
    Eq,
    PartialEq,
    Archive,
    Serialize,
    Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct UserPublicKey {
    pub user_pk: u64,
}

impl Ord for UserPublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.user_pk.cmp(&other.user_pk)
    }
}

/// License Nullifier.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialOrd,
    Eq,
    PartialEq,
    Archive,
    Serialize,
    Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseNullifier {
    pub value: u64,
}

impl Ord for LicenseNullifier {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

/// License Request.
#[derive(Debug, Clone, Eq, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseRequest {
    pub sp_public_key: SPPublicKey,
}

/// License Session.
#[derive(Debug, Clone, Eq, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseSession {
    pub nullifier: LicenseNullifier,
}

/// License.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct License {
    pub user_pk: UserPublicKey,
}
