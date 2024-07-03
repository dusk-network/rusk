// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::vec::Vec;

mod errors;
mod tx;

#[cfg(feature = "local_prover")]
pub mod prover;
#[cfg(feature = "local_prover")]
pub use crate::prover::LocalProver;

pub use errors::ProverError;
pub use tx::{UnprovenTransaction, UnprovenTransactionInput};

pub type ProverResult = Result<Vec<u8>, ProverError>;

pub trait Prover {
    fn prove_execute(&self, utx_bytes: &[u8]) -> ProverResult;
}
