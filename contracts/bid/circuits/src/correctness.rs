// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::{Error, Result};
use dusk_blindbid::{V_MAX, V_MIN};
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::{AllocatedScalar, RangeGadgets::range_check};

/// Circuit which proves the correctness of a blind bid.
#[derive(Debug, Clone, Default)]
pub struct CorrectnessCircuit {
    /// The value commitment of the bid.
    pub commitment: Option<AffinePoint>,
    /// The value of the bid, in clear.
    pub value: Option<BlsScalar>,
    /// The blinder, used to construct the value commitment.
    pub blinder: Option<BlsScalar>,
    /// The size of the circuit
    pub size: usize,
    /// Public input constructor, used when generating a proof.
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for CorrectnessCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<Vec<PublicInput>, Error> {
        let mut pi = vec![];

        // Make sure we have all of the circuit inputs before proceeding.
        let commitment = self
            .commitment
            .ok_or(CircuitErrors::CircuitInputsNotFound)?;
        let value = self.value.ok_or(CircuitErrors::CircuitInputsNotFound)?;
        let blinder =
            self.blinder.ok_or(CircuitErrors::CircuitInputsNotFound)?;

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

        self.size = composer.circuit_size();
        Ok(pi)
    }

    fn compile(
        &mut self,
        pub_params: &PublicParameters,
    ) -> Result<(ProverKey, VerifierKey, usize), Error> {
        // Setup PublicParams
        let (ck, _) = pub_params.trim(1 << 10)?;
        // Generate & save `ProverKey` with some random values.
        let mut prover = Prover::new(b"BidCorrectness");
        // Set size & PI builder
        self.pi_constructor = Some(self.gadget(prover.mut_cs())?);
        prover.preprocess(&ck)?;

        // Generate & save `VerifierKey` with some random values.
        let mut verifier = Verifier::new(b"BidCorrectness");
        self.gadget(verifier.mut_cs())?;
        verifier.preprocess(&ck)?;
        Ok((
            prover
                .prover_key
                .expect("Unexpected error. Missing VerifierKey in compilation"),
            verifier
                .verifier_key
                .expect("Unexpected error. Missing VerifierKey in compilation"),
            self.circuit_size(),
        ))
    }

    fn build_pi(&self, pub_inputs: &[PublicInput]) -> Result<Vec<BlsScalar>> {
        let mut pi = vec![BlsScalar::zero(); self.size];
        self.pi_constructor
            .as_ref()
            .ok_or(CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .enumerate()
            .for_each(|(idx, pi_constr)| {
                match pi_constr {
                    PublicInput::BlsScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::JubJubScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::AffinePoint(_, pos_x, pos_y) => {
                        let (coord_x, coord_y) = (
                            pub_inputs[idx].value()[0],
                            pub_inputs[idx].value()[1],
                        );
                        pi[*pos_x] = -coord_x;
                        pi[*pos_y] = -coord_y;
                    }
                };
            });
        Ok(pi)
    }

    fn circuit_size(&self) -> usize {
        self.size
    }

    fn gen_proof(
        &mut self,
        pub_params: &PublicParameters,
        prover_key: &ProverKey,
        transcript_initialisation: &'static [u8],
    ) -> Result<Proof> {
        let (ck, _) = pub_params.trim(1 << 10)?;
        // New Prover instance
        let mut prover = Prover::new(transcript_initialisation);
        // Fill witnesses for Prover
        self.gadget(prover.mut_cs())?;
        // Add ProverKey to Prover
        prover.prover_key = Some(prover_key.clone());
        prover.prove(&ck)
    }

    fn verify_proof(
        &mut self,
        pub_params: &PublicParameters,
        verifier_key: &VerifierKey,
        transcript_initialisation: &'static [u8],
        proof: &Proof,
        pub_inputs: &[PublicInput],
    ) -> Result<(), Error> {
        let (_, vk) = pub_params.trim(1 << 10)?;
        // New Verifier instance
        let mut verifier = Verifier::new(transcript_initialisation);
        // Fill witnesses for Verifier
        self.gadget(verifier.mut_cs())?;
        verifier.verifier_key = Some(*verifier_key);
        verifier.verify(proof, &vk, &self.build_pi(pub_inputs)?)
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
            commitment: Some(c),
            value: Some(value.into()),
            blinder: Some(blinder.into()),
            size: 0,
            pi_constructor: None,
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;

        let pi = vec![PublicInput::AffinePoint(c, 0, 0)];

        circuit.verify_proof(
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
            commitment: Some(c),
            value: Some(value.into()),
            blinder: Some(blinder.into()),
            size: 0,
            pi_constructor: None,
        };

        let pub_params =
            PublicParameters::setup(1 << 11, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"BidCorrectness")?;

        let pi = vec![PublicInput::AffinePoint(c, 0, 0)];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"BidCorrectness", &proof, &pi,)
            .is_err());
        Ok(())
    }
}
