// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;

use super::*;
use crate::error::Error;

use rand::rngs::OsRng;

impl RuskProver {
    pub fn prove_execute(&self, utx_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        info!("Received prove_execute request");
        let utx = UnprovenTransaction::from_slice(utx_bytes)
            .map_err(Error::Serialization)?;

        let mut circ =
            circuit_from_numbers(utx.inputs().len(), utx.outputs().len())
                .ok_or(Error::InvalidCircuitArguments(
                    utx.inputs().len(),
                    utx.outputs().len(),
                ))?;

        for input in utx.inputs() {
            let cis = CircuitInputSignature::from(input.signature());
            let cinput = CircuitInput::new(
                *input.opening(),
                *input.note(),
                input.pk_r_prime().into(),
                input.value(),
                input.blinding_factor(),
                input.nullifier(),
                cis,
            );

            circ.add_input(cinput);
        }

        for (note, value, blinder) in utx.outputs() {
            circ.add_output_with_data(*note, *value, *blinder);
        }

        circ.set_tx_hash(utx.hash());

        if let Some((crossover, value, blinder)) = utx.crossover() {
            circ.set_fee_crossover(utx.fee(), crossover, *value, *blinder);
        } else {
            circ.set_fee(utx.fee());
        }

        let keys = keys_for(circ.circuit_id())?;
        let pk = &keys.get_prover()?;

        let (proof, _) = circ.prove(&mut OsRng, pk).map_err(|e| {
            std::io::Error::new(
                io::ErrorKind::Other,
                format!("Failed proving the circuit: {e}"),
            )
        })?;

        Ok(proof.to_bytes().to_vec())
    }
}
