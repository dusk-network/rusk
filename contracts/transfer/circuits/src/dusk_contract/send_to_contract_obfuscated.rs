// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets::range::range;
use anyhow::{Error, Result};
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

/// XXX: missing fields from stuct as per gitbook. This is for one time encryption pk.
#[derive(Debug, Default, Clone)]
pub struct SendToContractObfuscatedCircuit {
    /// Value within commitment crossover
    pub commitment_crossover_value: BlsScalar,
    /// Blinder within commitment crossover
    pub commitment_crossover_blinder: BlsScalar,
    /// Commitment crossover point
    pub commitment_crossover: AffinePoint,
    /// Value within commitment message
    pub commitment_message_value: BlsScalar,
    /// Blinder within message commitment
    pub commitment_message_blinder: BlsScalar,
    /// Message commitment point
    pub commitment_message: AffinePoint,
    /// Returns circuit size
    pub trim_size: usize,
    /// Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for SendToContractObfuscatedCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let commitment_crossover_value = self.commitment_crossover_value;
        let commitment_crossover_blinder = self.commitment_crossover_blinder;
        let commitment_crossover = self.commitment_crossover;
        let commitment_message_value = self.commitment_message_value;
        let commitment_message_blinder = self.commitment_message_blinder;
        let commitment_message = self.commitment_message;
        let pi = self.get_mut_pi_positions();

        // Create allocated scalars for private inputs
        let allocated_crossover_value =
            AllocatedScalar::allocate(composer, commitment_crossover_value);
        let allocated_crossover_blinder =
            AllocatedScalar::allocate(composer, commitment_crossover_blinder);
        let allocated_message_value =
            AllocatedScalar::allocate(composer, commitment_message_value);
        let allocated_message_blinder =
            AllocatedScalar::allocate(composer, commitment_message_blinder);

        // Prove the knowledge of the commitment opening of the commitment of the crossover in the input
        let p1 = scalar_mul(
            composer,
            allocated_crossover_value.var,
            GENERATOR_EXTENDED,
        );
        let p2 = scalar_mul(
            composer,
            allocated_crossover_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

        let commitment = p1.point().fast_add(composer, *p2.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_crossover,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, commitment_crossover);

        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, allocated_crossover_value, 64);

        // Prove the knowledge of the commitment opening of the commitment of the crossover in the input
        let p3 = scalar_mul(
            composer,
            allocated_message_value.var,
            GENERATOR_EXTENDED,
        );
        let p4 = scalar_mul(
            composer,
            allocated_message_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

        let commitment2 = p3.point().fast_add(composer, *p4.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_message,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment2, commitment_message);

        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, allocated_message_value, 64);

        composer.assert_equal(
            allocated_crossover_value.var,
            allocated_message_value.var,
        );

        Ok(())
    }

    /// Returns the size at which we trim the `PublicParameters`
    /// to compile the circuit or perform proving/verification
    /// actions.
    fn get_trim_size(&self) -> usize {
        self.trim_size
    }

    fn set_trim_size(&mut self, size: usize) {
        self.trim_size = size;
    }

    /// /// Return a mutable reference to the Public Inputs storage of the circuit.
    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    /// Return a reference to the Public Inputs storage of the circuit.
    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;

    #[test]
    fn test_send_to_contract_obfuscated() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        let commitment_message_value = JubJubScalar::from(300 as u64);
        let commitment_message_blinder = JubJubScalar::from(200 as u64);
        let commitment_message = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_message_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_message_blinder),
        );

        // Build circuit structure
        let mut circuit = SendToContractObfuscatedCircuit {
            commitment_crossover_value: commitment_crossover_value.into(),
            commitment_crossover_blinder: commitment_crossover_blinder.into(),
            commitment_crossover: commitment_crossover,
            commitment_message_value: commitment_message_value.into(),
            commitment_message_blinder: commitment_message_blinder.into(),
            commitment_message: commitment_message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(commitment_message, 0, 0),
        ];

        circuit.verify_proof(&pub_params, &vk, b"ObfuscatedSend", &proof, &pi)
    }

    #[test]
    fn test_send_to_contract_obfuscated_wrong_value() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(200 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        let commitment_message_value = JubJubScalar::from(300 as u64);
        let commitment_message_blinder = JubJubScalar::from(200 as u64);
        let commitment_message = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_message_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_message_blinder),
        );

        // Build circuit structure
        let mut circuit = SendToContractObfuscatedCircuit {
            commitment_crossover_value: commitment_crossover_value.into(),
            commitment_crossover_blinder: commitment_crossover_blinder.into(),
            commitment_crossover: commitment_crossover,
            commitment_message_value: commitment_message_value.into(),
            commitment_message_blinder: commitment_message_blinder.into(),
            commitment_message: commitment_message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(commitment_message, 0, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedSend", &proof, &pi)
            .is_err());
        Ok(())
    }
}
