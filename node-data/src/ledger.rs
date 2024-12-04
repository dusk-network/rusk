// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod header;
pub use header::{Header, Seed};

mod block;
pub use block::*;

mod transaction;
pub use transaction::{SpendingId, SpentTransaction, Transaction};

mod faults;
pub use faults::{Fault, InvalidFault, Slash, SlashType};

mod attestation;
pub use attestation::{
    Attestation, IterationInfo, IterationsInfo, Signature, StepVotes,
};

use std::io::{self, Read, Write};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};
use sha3::Digest;

use crate::bls::PublicKeyBytes;
use crate::Serializable;

/// Encode a byte array into a shortened HEX representation.
pub fn to_str(bytes: &[u8]) -> String {
    const OFFSET: usize = 16;
    let hex = hex::encode(bytes);
    if bytes.len() <= OFFSET {
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
