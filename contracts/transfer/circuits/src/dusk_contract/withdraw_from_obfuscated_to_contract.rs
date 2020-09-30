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
    /// Spend Value within Pedersen commitment
    pub commitment_value: Option<BlsScalar>,
    /// Spend Blinder within Pedersen commitment
    pub commitment_blinder: Option<BlsScalar>,
    /// Spend Pedersen Commitment 
    pub commitment_point: Option<AffinePoint>,
    /// Message Value within Pedersen commitment
    pub spend_commitment_value: Option<BlsScalar>,
    /// Message Blinder within Pedersen commitment
    pub spend_commitment_blinder: Option<BlsScalar>,
    /// Message Pedersen Commitment 
    pub spend_commitment: Option<AffinePoint>,
    /// Note Value within Pedersen commitment
    pub change_commitment_value: Option<BlsScalar>,
    /// Note Blinder within Pedersen commitment
    pub change_commitment_blinder: Option<BlsScalar>,
    /// Note Pedersen Commitment 
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

        /// Create allocated scalars for private inputs
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
        
        let p1 = scalar_mul(composer, commitment_value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, commitment_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment = p1.point().fast_add(composer, *p2.point());
    
        pi.push(PublicInput::AffinePoint(
            commitment_point,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        composer.assert_equal_public_point(commitment, commitment_point);
        
        range(composer, commitment_value, 64);

        let p3 = scalar_mul(composer, spend_value.var, GENERATOR_EXTENDED);
        let p4 = scalar_mul(composer, spend_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment2 = p3.point().fast_add(composer, *p4.point());
    
        pi.push(PublicInput::AffinePoint(
            spend_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        composer.assert_equal_public_point(commitment2, spend_commitment);
        
        range(composer, spend_value, 64);

        let p5 = scalar_mul(composer, change_value.var, GENERATOR_EXTENDED);
        let p6 = scalar_mul(composer, change_blind.var, GENERATOR_NUMS_EXTENDED);
        
        let commitment3 = p5.point().fast_add(composer, *p6.point());
    
        pi.push(PublicInput::AffinePoint(
            change_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
    
        composer.assert_equal_public_point(commitment3, change_commitment);
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn test_withdraw_from_obfuscated_to_contract_one() -> Result<()> {
        
        let commitment_value = JubJubScalar::from(300 as u64);
        let commitment_blinder = JubJubScalar::from(100 as u64);
        let commitment_point = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_blinder),
        );

        let spend_value = JubJubScalar::from(200 as u64);
        let spend_blinder = JubJubScalar::from(200 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        let change_value = JubJubScalar::from(100 as u64);
        let change_blinder = JubJubScalar::from(300 as u64);
        let change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_value)
                + &(GENERATOR_NUMS_EXTENDED * change_blinder),
        );

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
}



pub fn withdraw_from_obfuscated_to_contract_one(
    composer: &mut StandardComposer,
    commitment_one: AffinePoint,
    commitment_one_value: AllocatedScalar,
    commitment_one_blinder: AllocatedScalar,
    message_spend_commitment: AffinePoint,
    message_spend_commitment_value: AllocatedScalar, 
    message_spend_commitment_blinder: AllocatedScalar,
    message_change_commitment: AffinePoint,
    message_change_commitment_value: AllocatedScalar,
    message_change_commitment_blinder: AllocatedScalar,
) {

    commitment(composer, commitment_one_value, commitment_one_blinder, commitment_one);
    commitment(composer, message_spend_commitment_value, message_spend_commitment_blinder, message_spend_commitment);
    commitment(composer, message_change_commitment_value, message_change_commitment_blinder, message_change_commitment);

    range(composer, commitment_one_value, 64);
    range(composer, message_spend_commitment_value, 64);
    range(composer, message_change_commitment_value, 64);

    composer.add_gate(
        message_spend_commitment_value.var,
        message_change_commitment_value.var,
        commitment_one_value.var,
        BlsScalar::one(),
        BlsScalar::one(),
        -BlsScalar::one(),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );
}

pub fn withdraw_from_obfuscated_to_contract_two(
    composer: &mut StandardComposer,
    commitment_two: AffinePoint,
    commitment_two_value: AllocatedScalar,
    commitment_two_blinder: AllocatedScalar,
    change_message_commitment: AffinePoint,
    change_message_commitment_value: AllocatedScalar, 
    change_message_commitment_blinder: AllocatedScalar,
    value: AllocatedScalar,
) {
    commitment(composer, commitment_two_value, commitment_two_blinder, commitment_two);
    commitment(composer, change_message_commitment_value, change_message_commitment_blinder, change_message_commitment);

    range(composer, commitment_two_value, 64);
    range(composer, change_message_commitment_value, 64);

    composer.add_gate(
        value.var,
        change_message_commitment_value.var,
        commitment_two_value.var,
        BlsScalar::one(),
        BlsScalar::one(),
        -BlsScalar::one(),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use anyhow::{Error, Result};
//     use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
//     use dusk_plonk::proof_system::{Prover, Verifier};

//     #[test]
//     fn obfuscated_to_contract_one() -> Result<(), Error> {

//         let commitment_one_value = JubJubScalar::from(300 as u64);
//         let commitment_one_blinder = JubJubScalar::from(100 as u64);
//         let commitment_one = AffinePoint::from(
//             &(GENERATOR_EXTENDED * commitment_one_value)
//                 + &(GENERATOR_NUMS_EXTENDED * commitment_one_blinder),
//         );

//         let message_spend_commitment_value = JubJubScalar::from(200 as u64);
//         let message_spend_commitment_blinder = JubJubScalar::from(200 as u64);
//         let message_spend_commitment = AffinePoint::from(
//             &(GENERATOR_EXTENDED * message_spend_commitment_value)
//                 + &(GENERATOR_NUMS_EXTENDED * message_spend_commitment_blinder),
//         );

//         let message_change_commitment_value = JubJubScalar::from(100 as u64);
//         let message_change_commitment_blinder = JubJubScalar::from(300 as u64);
//         let message_change_commitment = AffinePoint::from(
//             &(GENERATOR_EXTENDED * message_change_commitment_value)
//                 + &(GENERATOR_NUMS_EXTENDED * message_change_commitment_blinder),
//         );

//         // Generate Composer & Public Parameters
//         let pub_params =
//             PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
//         let (ck, vk) = pub_params.trim(1 << 16)?;
//         let mut prover = Prover::new(b"test");


//         let commitment_one_value =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
//         let commitment_one_blinder =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
//         let message_spend_commitment_value =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
//         let message_spend_commitment_blinder =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
//         let message_change_commitment_value =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
//         let message_change_commitment_blinder =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));

//         withdraw_from_obfuscated_to_contract_one(
//             prover.mut_cs(),
//             commitment_one,
//             commitment_one_value, 
//             commitment_one_blinder, 
//             message_spend_commitment,
//             message_spend_commitment_value, 
//             message_spend_commitment_blinder,
//             message_change_commitment, 
//             message_change_commitment_value, 
//             message_change_commitment_blinder,
//         );

//         prover.preprocess(&ck)?;
//         let proof = prover.prove(&ck)?;

        
//         let mut verifier = Verifier::new(b"test");

//         let commitment_one_value =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
//         let commitment_one_blinder =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
//         let message_spend_commitment_value =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
//         let message_spend_commitment_blinder =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
//         let message_change_commitment_value =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
//         let message_change_commitment_blinder =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        
//         withdraw_from_obfuscated_to_contract_one(
//             verifier.mut_cs(),
//             commitment_one,
//             commitment_one_value, 
//             commitment_one_blinder, 
//             message_spend_commitment,
//             message_spend_commitment_value, 
//             message_spend_commitment_blinder,
//             message_change_commitment, 
//             message_change_commitment_value, 
//             message_change_commitment_blinder,
//         );
        
//         verifier.preprocess(&ck)?;

//         let pi = verifier.mut_cs().public_inputs.clone();
//         verifier.verify(&proof, &vk, &pi)
//     }

//     #[test]
//     fn obfuscated_to_contract_two() -> Result<(), Error> {

//         let commitment_two_value = JubJubScalar::from(300 as u64);
//         let commitment_two_blinder = JubJubScalar::from(100 as u64);
//         let commitment_two = AffinePoint::from(
//             &(GENERATOR_EXTENDED * commitment_two_value)
//                 + &(GENERATOR_NUMS_EXTENDED * commitment_two_blinder),
//         );

//         let change_message_commitment_value = JubJubScalar::from(200 as u64);
//         let change_message_commitment_blinder = JubJubScalar::from(200 as u64);
//         let change_message_commitment = AffinePoint::from(
//             &(GENERATOR_EXTENDED * change_message_commitment_value)
//                 + &(GENERATOR_NUMS_EXTENDED * change_message_commitment_blinder),
//         );


//         // Generate Composer & Public Parameters
//         let pub_params =
//             PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
//         let (ck, vk) = pub_params.trim(1 << 16)?;
//         let mut prover = Prover::new(b"test");

//         let commitment_two_value =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
//         let commitment_two_blinder =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
//         let change_message_commitment_value =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
//         let change_message_commitment_blinder =
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
//         let value = 
//             AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        
//             withdraw_from_obfuscated_to_contract_two(
//             prover.mut_cs(),
//             commitment_two,
//             commitment_two_value,
//             commitment_two_blinder,
//             change_message_commitment,
//             change_message_commitment_value,
//             change_message_commitment_blinder,
//             value,
//         );

//         prover.preprocess(&ck)?;
//         let proof = prover.prove(&ck)?;

        
//         let mut verifier = Verifier::new(b"test");

//         let commitment_two_value =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
//         let commitment_two_blinder =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
//         let change_message_commitment_value =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
//         let change_message_commitment_blinder =
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
//         let value = 
//             AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        
//             withdraw_from_obfuscated_to_contract_two(
//             verifier.mut_cs(),
//             commitment_two,
//             commitment_two_value,
//             commitment_two_blinder,
//             change_message_commitment,
//             change_message_commitment_value,
//             change_message_commitment_blinder,
//             value,
//         );

//         verifier.preprocess(&ck)?;

//         let pi = verifier.mut_cs().public_inputs.clone();
//         verifier.verify(&proof, &vk, &pi)
//     }

// }