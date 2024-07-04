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

mod faults;
pub use faults::Fault;

mod attestation;
pub use attestation::{
    Attestation, IterationInfo, IterationsInfo, Signature, StepVotes,
};

use crate::bls::PublicKeyBytes;
use crate::Serializable;

use dusk_bytes::DeserializableSlice;
use rusk_abi::hash::Hasher;
use sha3::Digest;
use std::io::{self, Read, Write};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

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
    pub use super::transaction::faker::gen_dummy_tx;
}
