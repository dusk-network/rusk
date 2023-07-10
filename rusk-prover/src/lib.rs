// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod errors;

#[cfg(feature = "local_prover")]
pub mod prover;
#[cfg(feature = "local_prover")]
pub use crate::prover::LocalProver;

pub use errors::ProverError;

pub type ProverResult = Result<Vec<u8>, ProverError>;

pub trait Prover {
    fn prove_execute(&self, utx_bytes: &[u8]) -> ProverResult;
    fn prove_stco(&self, circuit_inputs: &[u8]) -> ProverResult;
    fn prove_stct(&self, circuit_inputs: &[u8]) -> ProverResult;
    fn prove_wfco(&self, circuit_inputs: &[u8]) -> ProverResult;
    fn prove_wfct(&self, circuit_inputs: &[u8]) -> ProverResult;
}
