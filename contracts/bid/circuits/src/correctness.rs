// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use dusk_blindbid::{V_MAX, V_MIN};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    JubJubAffine as AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::{AllocatedScalar, RangeGadgets::range_check};

/// Circuit which proves the correctness of a blind bid.
#[derive(Debug, Clone, Default)]
pub struct CorrectnessCircuit {
    /// The value commitment of the bid.
    pub commitment: AffinePoint,
    /// The value of the bid, in clear.
    pub value: BlsScalar,
    /// The blinder, used to construct the value commitment.
    pub blinder: BlsScalar,
    /// The size of the circuit
    pub trim_size: usize,
    /// Public input constructor, used when generating a proof.
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for CorrectnessCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let commitment = self.commitment;
        let value = self.value;
        let blinder = self.blinder;

        let pi = self.get_mut_pi_positions();
        // Allocate all private inputs to the circuit.
        let value = AllocatedScalar::allocate(composer, value);
        let blinder = AllocatedScalar::allocate(composer, blinder);

        // ------------------------------------------------------- //
        //                                                         //
        //                   Correctness circuit                   //
        //                                                         //
        // ------------------------------------------------------- //

        // 1. Prove knowledge of commitment pre-image.
        let p1 = scalar_mul(composer, value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, blinder.var, GENERATOR_NUMS_EXTENDED);
        let computed_commitment = p1.point().fast_add(composer, *p2.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Ensure equality between the computed commitment and the provided one.
        composer.assert_equal_public_point(computed_commitment, commitment);

        // 2. Range check - v_min <= value <= v_max
        let cond = range_check(
            composer,
            BlsScalar::from(*V_MIN),
            BlsScalar::from(*V_MAX),
            value,
        );

        // Constrain cond to be one - meaning that the range check holds.
        composer.constrain_to_constant(
            cond,
            BlsScalar::one(),
            BlsScalar::zero(),
        );

        Ok(())
    }

    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }

    fn get_trim_size(&self) -> usize {
        self.trim_size
    }

    fn set_trim_size(&mut self, size: usize) {
        self.trim_size = size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusk_plonk::jubjub::AffinePoint as JubJubAffine;

    #[test]
    fn test_correctness_circuit() -> Result<()> {
        let value = JubJubScalar::from(100000 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = JubJubAffine::from(
            (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
        );

        let mut circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
            trim_size: 1 << 10,
            pi_positions: vec![],
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;

        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;

        let mut verifier_circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
            trim_size: 1 << 10,
            pi_positions: vec![],
        };
        let pi = vec![PublicInput::AffinePoint(c, 0, 0)];

        verifier_circuit.verify_proof(
            &pub_params,
            &vk,
            b"BidCorrectness",
            &proof,
            &pi,
        )?;
        Ok(())
    }

    #[test]
    fn test_correctness_circuit_out_of_bounds() -> Result<()> {
        let value = JubJubScalar::from(100 as u64);
        let blinder = JubJubScalar::from(50000 as u64);

        let c = JubJubAffine::from(
            (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
        );

        let mut circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
            trim_size: 1 << 10,
            pi_positions: vec![],
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;

        let mut verifier_circuit = CorrectnessCircuit {
            commitment: c,
            value: value.into(),
            blinder: blinder.into(),
            trim_size: 1 << 10,
            pi_positions: vec![],
        };
        let pi = vec![PublicInput::AffinePoint(c, 0, 0)];

        assert!(verifier_circuit
            .verify_proof(&pub_params, &vk, b"BidCorrectness", &proof, &pi,)
            .is_err());
        Ok(())
    }
}
