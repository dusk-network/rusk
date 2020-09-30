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
/// for a 'withdraw from obfuscated contract' transaction.
#[derive(Debug, Default, Clone)]
pub struct WithdrawFromObfuscatedToContractCircuitOne {
    /// Spend Value within commitment
    pub commitment_value: Option<BlsScalar>,
    /// Spend Blinder within commitment
    pub commitment_blinder: Option<BlsScalar>,
    /// Spend Commitment 
    pub commitment_point: Option<AffinePoint>,
    /// Message Value within spend commitment
    pub spend_commitment_value: Option<BlsScalar>,
    /// Message Blinder within spend commitment
    pub spend_commitment_blinder: Option<BlsScalar>,
    /// Message spend Commitment 
    pub spend_commitment: Option<AffinePoint>,
    /// Note Value within change commitment
    pub change_commitment_value: Option<BlsScalar>,
    /// Note Blinder within change commitment
    pub change_commitment_blinder: Option<BlsScalar>,
    /// Note change Commitment 
    pub change_commitment: Option<AffinePoint>,
    // Returns circuit size
    pub size: usize,
    // Gives Public Inputs
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for WithdrawFromObfuscatedToContractCircuitOne {

    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<Vec<PublicInput>, Error> {
        let mut pi: Vec<PublicInput> = vec![];
        let commitment_value = self.commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_blinder = self.commitment_blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_point = self.commitment_point.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let spend_commitment_value = self.spend_commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let spend_commitment_blinder = self.spend_commitment_blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let spend_commitment = self.spend_commitment.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment_value = self.change_commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment_blinder = self.change_commitment_blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment = self.change_commitment.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;

        // Create allocated scalars for private inputs
        let commitment_value =
            AllocatedScalar::allocate(composer, commitment_value);
        let commitment_blind =
            AllocatedScalar::allocate(composer, commitment_blinder);
        let spend_value =
            AllocatedScalar::allocate(composer, spend_commitment_value);
        let spend_blind =
            AllocatedScalar::allocate(composer, spend_commitment_blinder);
        let change_value =
            AllocatedScalar::allocate(composer, change_commitment_value);
        let change_blind =
            AllocatedScalar::allocate(composer, change_commitment_blinder);
        
        // Prove the knowledge of the commitment opening of the commitment of the input
        let p1 = scalar_mul(composer, commitment_value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, commitment_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment = p1.point().fast_add(composer, *p2.point());
    
        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_point,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, commitment_point);
         
        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, commitment_value, 64);

        // Prove the knowledge of the spend commitment opening of the commitment of the input
        let p3 = scalar_mul(composer, spend_value.var, GENERATOR_EXTENDED);
        let p4 = scalar_mul(composer, spend_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment2 = p3.point().fast_add(composer, *p4.point());
    
        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            spend_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment2, spend_commitment);
        
        // Prove that the value of the opening of the spend commitment of the input is within range
        range(composer, spend_value, 64);

        // Prove the knowledge of the change commitment opening of the commitment of the input
        let p5 = scalar_mul(composer, change_value.var, GENERATOR_EXTENDED);
        let p6 = scalar_mul(composer, change_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment3 = p5.point().fast_add(composer, *p6.point());
    
        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            change_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment3, change_commitment);
        
        // Prove that the value of the opening of the change commitment of the input is within range
        range(composer, change_value, 64);

        composer.add_gate(
            spend_value.var,
            change_value.var,
            commitment_value.var,
            BlsScalar::one(),
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
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

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'withdraw from obfuscated contract' transaction.
/// This is the second declared ciruit.
#[derive(Debug, Default, Clone)]
pub struct WithdrawFromObfuscatedToContractCircuitTwo {
    /// Spend Value within Pedersen commitment
    pub commitment_value: Option<BlsScalar>,
    /// Spend Blinder within Pedersen commitment
    pub commitment_blinder: Option<BlsScalar>,
    /// Spend Pedersen Commitment 
    pub commitment_point: Option<AffinePoint>,
    /// Note Value within Pedersen commitment
    pub change_commitment_value: Option<BlsScalar>,
    /// Note Blinder within Pedersen commitment
    pub change_commitment_blinder: Option<BlsScalar>,
    /// Note Pedersen Commitment 
    pub change_commitment: Option<AffinePoint>,
    /// Value to be sent
    pub value: Option<BlsScalar>,
    // Returns circuit size
    pub size: usize,
    // Gives Public Inputs
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for WithdrawFromObfuscatedToContractCircuitTwo {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<Vec<PublicInput>, Error> {
        let mut pi: Vec<PublicInput> = vec![];
        let commitment_value = self.commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_blinder = self.commitment_blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_point = self.commitment_point.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment_value = self.change_commitment_value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment_blinder = self.change_commitment_blinder.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let change_commitment = self.change_commitment.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let value = self.value.ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;

        // Create allocated scalars for private inputs
        let allocated_commitment_value = AllocatedScalar::allocate(composer, commitment_value);
        let allocated_commitment_blinder = AllocatedScalar::allocate(composer, commitment_blinder);
        let allocated_change_value = AllocatedScalar::allocate(composer, change_commitment_value);
        let allocated_change_blinder = AllocatedScalar::allocate(composer, change_commitment_blinder);

        // Allow adding of zero into circuit as a variable
        let zero = composer.add_witness_to_circuit_description(BlsScalar::zero());

        // Prove the knowledge of the commitment opening of the commitment of the input
        let p1 = scalar_mul(composer, allocated_commitment_value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, allocated_commitment_blinder.var, GENERATOR_NUMS_EXTENDED);
    
        let commitment = p1.point().fast_add(composer, *p2.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_point,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, commitment_point);
    
        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, allocated_commitment_value, 64);

        // Prove the knowledge of the change commitment opening of the commitment of the input
        let p3 = scalar_mul(composer, allocated_change_value.var, GENERATOR_EXTENDED);
        let p4 = scalar_mul(composer, allocated_change_blinder.var, GENERATOR_NUMS_EXTENDED);
    
        let commitment = p3.point().fast_add(composer, *p4.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            change_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, change_commitment);
    
        // Prove that the value of the opening of the change commitment of the input is within range
        range(composer, allocated_change_value, 64);

         // Add PI constraint for the sum check
        pi.push(PublicInput::BlsScalar(
            -value,
            composer.circuit_size(),
        ));

        // Constrain: value - change value - commitment value = 0 
        composer.poly_gate(
            allocated_change_value.var,
            allocated_commitment_value.var,
            zero,
            BlsScalar::zero(),
            BlsScalar::one(),
            BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            -value,
        );

        // Set the final circuit size as a Circuit struct attribute.
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
    fn test_withdraw_from_obfuscated_to_contract_one() -> Result<()> {
        
        // Define and create commitment values
        let commitment_value = JubJubScalar::from(300 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        // Define and create spend commitment values
        let spend_value = JubJubScalar::from(200 as u64);
        let spend_blinder = JubJubScalar::from(200 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        // Define and create change commitment values
        let change_value = JubJubScalar::from(100 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

        // Build circuit structure
        let mut circuit = WithdrawFromObfuscatedToContractCircuitOne {
            commitment_value: Some(commitment_value.into()),
            commitment_blinder: Some(commitment_blinder.into()),
            commitment_point: Some(commitment_point),
            spend_commitment_value: Some(spend_value.into()),
            spend_commitment_blinder: Some(spend_blinder.into()),
            spend_commitment: Some(spend_commitment),
            change_commitment_value: Some(change_value.into()),
            change_commitment_blinder: Some(change_blinder.into()),
            change_commitment: Some(change_commitment),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawOne")?;  

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
        ];

        circuit.verify_proof(&pub_params, &vk, b"ObfuscatedWithdrawOne", &proof, &pi)
    }

    #[test]
    fn test_withdraw_from_obfuscated_to_contract_one_wrong_value() -> Result<()> {
        
        // Define and create commitment values
        let commitment_value = JubJubScalar::from(400 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        // Define and create spend commitment values
        let spend_value = JubJubScalar::from(200 as u64);
        let spend_blinder = JubJubScalar::from(200 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        // Define and create change commitment values
        let change_value = JubJubScalar::from(100 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

        // Build circuit structure
        let mut circuit = WithdrawFromObfuscatedToContractCircuitOne {
            commitment_value: Some(commitment_value.into()),
            commitment_blinder: Some(commitment_blinder.into()),
            commitment_point: Some(commitment_point),
            spend_commitment_value: Some(spend_value.into()),
            spend_commitment_blinder: Some(spend_blinder.into()),
            spend_commitment: Some(spend_commitment),
            change_commitment_value: Some(change_value.into()),
            change_commitment_blinder: Some(change_blinder.into()),
            change_commitment: Some(change_commitment),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawOne")?;  

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedWithdrawOne", &proof, &pi)
            .is_err());
        Ok(())
    }


    #[test]
    fn test_withdraw_from_obfuscated_to_contract_two() -> Result<()> {
        
        // Define and create commitment values
        let commitment_value = JubJubScalar::from(300 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        // Define and create change commitment values
        let change_value = JubJubScalar::from(100 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(400 as u64);

        // Build circuit structure
        let mut circuit = WithdrawFromObfuscatedToContractCircuitTwo {
            commitment_value: Some(commitment_value.into()),
            commitment_blinder: Some(commitment_blinder.into()),
            commitment_point: Some(commitment_point),
            change_commitment_value: Some(change_value.into()),
            change_commitment_blinder: Some(change_blinder.into()),
            change_commitment: Some(change_commitment),
            value: Some(value),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;  

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(-value, 0),
        ];

        circuit.verify_proof(&pub_params, &vk, b"ObfuscatedWithdrawTwo", &proof, &pi)
    }

    #[test]
    fn test_withdraw_from_obfuscated_to_contract_two_wrong_value() -> Result<()> {
        
        // Define and create commitment values
        let commitment_value = JubJubScalar::from(300 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        // Define and create change commitment values
        let change_value = JubJubScalar::from(500 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(400 as u64);

        // Build circuit structure
        let mut circuit = WithdrawFromObfuscatedToContractCircuitTwo {
            commitment_value: Some(commitment_value.into()),
            commitment_blinder: Some(commitment_blinder.into()),
            commitment_point: Some(commitment_point),
            change_commitment_value: Some(change_value.into()),
            change_commitment_blinder: Some(change_blinder.into()),
            change_commitment: Some(change_commitment),
            value: Some(value),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;  

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(-value, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedWithdrawTwo", &proof, &pi)
            .is_err());
        Ok(())
    }


    #[test]
    fn test_withdraw_from_obfuscated_to_contract_two_wrong_pi() -> Result<()> {
        
        // Define and create commitment values
        let commitment_value = JubJubScalar::from(300 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        // Define and create change commitment values
        let change_value = JubJubScalar::from(100 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(200 as u64);

        // Build circuit structure
        let mut circuit = WithdrawFromObfuscatedToContractCircuitTwo {
            commitment_value: Some(commitment_value.into()),
            commitment_blinder: Some(commitment_blinder.into()),
            commitment_point: Some(commitment_point),
            change_commitment_value: Some(change_value.into()),
            change_commitment_blinder: Some(change_blinder.into()),
            change_commitment: Some(change_commitment),
            value: Some(value),
            size: 0, 
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;  

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(-value, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedWithdrawTwo", &proof, &pi)
            .is_err());
        Ok(())
    }
}








