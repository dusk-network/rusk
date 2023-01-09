// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Signatures messages used in the stake contract.

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::APK;
use dusk_bytes::Serializable;
use dusk_pki::StealthAddress;
use phoenix_core::Note;

/// Return the digest to be signed in the `allow` function of the stake
/// contract.
#[must_use]
pub fn allow_sign_digest(counter: u64, staker: APK) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(u64::SIZE + APK::SIZE);

    bytes.extend(counter.to_bytes());
    bytes.extend(staker.to_bytes());

    bytes
}

/// Return the digest to be signed in the `stake` function of the stake
/// contract.
#[must_use]
pub fn stake_sign_digest(counter: u64, value: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16);

    bytes.extend(counter.to_bytes());
    bytes.extend(value.to_bytes());

    bytes
}

/// Return the digest to be signed in the `unstake` function of the stake
/// contract.
#[must_use]
pub fn unstake_sign_digest(counter: u64, note: Note) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(u64::SIZE + Note::SIZE);

    bytes.extend(counter.to_bytes());
    bytes.extend(note.to_bytes());

    bytes
}

/// Return the digest to be signed in the `withdraw` function of the stake
/// contract.
#[must_use]
pub fn withdraw_sign_digest(
    counter: u64,
    address: StealthAddress,
    nonce: BlsScalar,
) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(u64::SIZE + StealthAddress::SIZE + BlsScalar::SIZE);

    bytes.extend(counter.to_bytes());
    bytes.extend(address.to_bytes());
    bytes.extend(nonce.to_bytes());

    bytes
}
