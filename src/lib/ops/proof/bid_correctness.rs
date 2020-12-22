// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bid_circuits::CorrectnessCircuit;
use dusk_plonk::prelude::*;
use wasmi::{RuntimeValue, Trap};

pub(crate) fn bid_correctness_verification(
    pub_inputs: &Vec<PublicInput>,
    vk: &VerifierKey,
    proof: &Proof,
) -> Result<Option<RuntimeValue>, Trap> {
    let mut circuit = CorrectnessCircuit::default();
    circuit.set_trim_size(1 << 10);
    match circuit.verify_proof(
        &crate::PUB_PARAMS,
        &vk,
        b"BidCorrectness",
        &proof,
        &pub_inputs,
    ) {
        Ok(()) => Ok(Some(RuntimeValue::I32(1i32))),
        Err(_) => Ok(Some(RuntimeValue::I32(0i32))),
    }
}
