// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Signatures messages used in the contract which supports subsidizing.

use dusk_bytes::Serializable;

const SUBSIDY_MESSAGE_SIZE: usize = u64::SIZE + u64::SIZE;

/// Return the digest to be signed in the `subsidize` function of a contract.
#[must_use]
pub fn subsidy_signature_message(
    counter: u64,
    value: u64,
) -> [u8; SUBSIDY_MESSAGE_SIZE] {
    let mut bytes = [0u8; SUBSIDY_MESSAGE_SIZE];

    bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
    bytes[u64::SIZE..].copy_from_slice(&value.to_bytes());

    bytes
}
