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
pub use transaction::{SpentTransaction, Transaction, TransactionType};

mod faults;
pub use faults::{Fault, InvalidFault, Slash, SlashType};

mod attestation;
pub use attestation::{
    Attestation, IterationInfo, IterationsInfo, Signature, StepVotes,
};

use crate::bls::PublicKeyBytes;
use crate::Serializable;

use sha3::Digest;
use std::io::{self, Read, Write};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

/// Encode a byte array into a shortened HEX representation.
pub fn to_str<const N: usize>(bytes: &[u8; N]) -> String {
    const OFFSET: usize = 16;
    let hex = hex::encode(bytes);
    if N <= OFFSET {
        return hex;
    }

    let len = hex.len();

    let first = &hex[0..OFFSET];
    let last = &hex[len - OFFSET..];

    format!("{first}...{last}")
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    pub use super::transaction::faker::gen_dummy_tx;
}
