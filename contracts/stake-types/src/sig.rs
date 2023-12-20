// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Signatures messages used in the stake contract.

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_pki::StealthAddress;

const ALLOW_MESSAGE_SIZE: usize = u64::SIZE + PublicKey::SIZE;
const STAKE_MESSAGE_SIZE: usize = u64::SIZE + u64::SIZE;
const WITHDRAW_MESSAGE_SIZE: usize =
    u64::SIZE + StealthAddress::SIZE + BlsScalar::SIZE;

/// Signature message used for [`Allow`].
#[must_use]
pub fn allow_signature_message(
    counter: u64,
    staker: &PublicKey,
) -> [u8; ALLOW_MESSAGE_SIZE] {
    let mut bytes = [0u8; ALLOW_MESSAGE_SIZE];

    bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
    bytes[u64::SIZE..].copy_from_slice(&staker.to_bytes());

    bytes
}

/// Return the digest to be signed in the `stake` function of the stake
/// contract.
#[must_use]
pub fn stake_signature_message(
    counter: u64,
    value: u64,
) -> [u8; STAKE_MESSAGE_SIZE] {
    let mut bytes = [0u8; STAKE_MESSAGE_SIZE];

    bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
    bytes[u64::SIZE..].copy_from_slice(&value.to_bytes());

    bytes
}

/// Signature message used for [`Unstake`].
pub fn unstake_signature_message<T>(counter: u64, note: T) -> Vec<u8>
where
    T: AsRef<[u8]>,
{
    let mut vec = Vec::new();

    vec.extend_from_slice(&counter.to_bytes());
    vec.extend_from_slice(note.as_ref());

    vec
}

/// Signature message used for [`Withdraw`].
#[must_use]
pub fn withdraw_signature_message(
    counter: u64,
    address: StealthAddress,
    nonce: BlsScalar,
) -> [u8; WITHDRAW_MESSAGE_SIZE] {
    let mut bytes = [0u8; WITHDRAW_MESSAGE_SIZE];

    bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
    bytes[u64::SIZE..u64::SIZE + StealthAddress::SIZE]
        .copy_from_slice(&address.to_bytes());
    bytes[u64::SIZE + StealthAddress::SIZE..]
        .copy_from_slice(&nonce.to_bytes());

    bytes
}
