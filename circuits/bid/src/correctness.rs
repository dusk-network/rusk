// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_blindbid::{V_RAW_MAX, V_RAW_MIN};
use dusk_plonk::jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_plonk::prelude::*;
use plonk_gadgets::{AllocatedScalar, RangeGadgets::range_check};

/// Circuit which proves the correctness of a blind bid.
#[derive(Debug, Clone, Default)]
pub struct CorrectnessCircuit {
    /// The value commitment of the bid.
    pub commitment: JubJubAffine,
    /// The value of the bid, in clear.
    pub value: BlsScalar,
    /// The blinder, used to construct the value commitment.
    pub blinder: BlsScalar,
}

impl Circuit for CorrectnessCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<(), Error> {
        // Allocate all private inputs to the circuit.
        let value = AllocatedScalar::allocate(composer, self.value);
        let blinder = AllocatedScalar::allocate(composer, self.blinder);

        // ------------------------------------------------------- //
        //                                                         //
        //                   Correctness circuit                   //
        //                                                         //
        // ------------------------------------------------------- //

        // 1. Prove knowledge of commitment pre-image.
        let p1 = composer.fixed_base_scalar_mul(value.var, GENERATOR_EXTENDED);
        let p2 = composer
            .fixed_base_scalar_mul(blinder.var, GENERATOR_NUMS_EXTENDED);
        let computed_commitment = composer.point_addition_gate(p1, p2);

        // Ensure equality between the computed commitment and the provided one.
        composer
            .assert_equal_public_point(computed_commitment, self.commitment);

        // 2. Range check - v_min <= value <= v_max
        let cond = range_check(
            composer,
            BlsScalar::from(V_RAW_MIN),
            BlsScalar::from(V_RAW_MAX),
            value,
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusk_plonk::jubjub::JubJubAffine;

    #[test]
    fn test_correctness_circuit() -> Result<(), Error> {
        let value = JubJubScalar::from(100000 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = JubJubAffine::from(
            (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
        );

        let mut circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vd) = circuit.compile(&pub_params)?;

        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;
        let pi = vec![c.into()];
        circuit::verify_proof(
            &pub_params,
            &vd.key(),
            &proof,
            &pi,
            &vd.pi_pos(),
            b"BidCorrectness",
        )?;
        Ok(())
    }

    #[test]
    fn test_correctness_circuit_out_of_bounds() -> Result<(), Error> {
        let value = JubJubScalar::from(100 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = JubJubAffine::from(
            (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
        );

        let mut circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vd) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;

        let pi = vec![c.into()];
        assert!(circuit::verify_proof(
            &pub_params,
            &vd.key(),
            &proof,
            &pi,
            &vd.pi_pos(),
            b"BidCorrectness",
        )
        .is_err());
        Ok(())
    }
}
