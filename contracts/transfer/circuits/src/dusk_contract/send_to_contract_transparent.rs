// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::prelude::*;
use crate::gadgets::{range::range, commitment::commitment};
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED 
};
use plonk_gadgets::AllocatedScalar;
use anyhow::{Error, Result};

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Default, Clone)]
pub struct SendToContractTransparentCircuit {
    /// Value within Pedersen commitment
    pub commitment_value: Option<BlsScalar>,
    /// Blinder within Pedersen commitment
    pub blinder: Option<BlsScalar>,
    /// Pedersen Commitment 
    pub commitment: Option<AffinePoint>,
    /// Value to be sent
    pub value: Option<BlsScalar>,
    // Returns circuit size
    pub size: usize,
    // Gives Public Inputs
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for SendToContractTransparentCircuit {

    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<Vec<PublicInput>, Error> {
        let mut pi: Vec<PublicInput> = vec![];
        let commitment_crossover = self.commitment.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_crossover_value = self.commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_crossover_blinder = self.blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let value = self.value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;

        let allocated_commitment_crossover_value = AllocatedScalar::allocate(composer, commitment_crossover_value);
        let allocated_commitment_crossover_blinder = AllocatedScalar::allocate(composer, commitment_crossover_blinder);

        let p1 = scalar_mul(composer, allocated_commitment_crossover_value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, allocated_commitment_crossover_blinder.var, GENERATOR_NUMS_EXTENDED);
    
        let commitment = p1.point().fast_add(composer, *p2.point());

        pi.push(PublicInput::AffinePoint(
            commitment_crossover,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        composer.assert_equal_public_point(commitment, commitment_crossover);
    
        range(composer, allocated_commitment_crossover_value, 64);

        composer.constrain_to_constant(allocated_commitment_crossover_value.var, BlsScalar::zero(), -value);
        
        pi.push(PublicInput::BlsScalar(
            -value,
            composer.circuit_size(),
        ));

        self.size = composer.circuit_size();
        Ok(pi)
    }


    fn compile(
        &mut self,
        pub_params: &PublicParameters,
    ) -> Result<(ProverKey, VerifierKey, usize), Error> {
        // Setup PublicParams
        let (ck, _) = pub_params.trim(1 << 16)?;
        // Generate & save `ProverKey` with some random values.
        let mut prover = Prover::new(b"TestCircuit");
        // Set size & Pi builder
        self.pi_constructor = Some(self.gadget(prover.mut_cs())?);
        prover.preprocess(&ck)?;

        // Generate & save `VerifierKey` with some random values.
        let mut verifier = Verifier::new(b"TestCircuit");
        self.gadget(verifier.mut_cs())?;
        verifier.preprocess(&ck)?;
        Ok((
            prover
                .prover_key
                .expect("Unexpected error. Missing VerifierKey in compilation")
                .clone(),
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
        let (ck, _) = pub_params.trim(1 << 16)?;
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
        let (_, vk) = pub_params.trim(1 << 16)?;
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
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn test_send_to_contract_transparent() -> Result<()> {

        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );
        let value = BlsScalar::from(300);

        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: Some(commitment_crossover_value.into()),
            blinder: Some(commitment_crossover_blinder.into()),
            commitment: Some(commitment_crossover), 
            value: Some(value),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"TransparentSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::BlsScalar(-value, 0),
        ];

        circuit.verify_proof(&pub_params, &vk, b"TransparentSend", &proof, &pi)

    }
}
    
