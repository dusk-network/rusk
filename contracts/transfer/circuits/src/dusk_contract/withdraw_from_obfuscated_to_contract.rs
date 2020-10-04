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

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'withdraw from obfuscated contract' transaction.
#[derive(Debug, Default, Clone)]
pub struct WithdrawFromObfuscatedToContractCircuitOne {
    /// Spend Value within commitment
    pub commitment_value: BlsScalar,
    /// Spend Blinder within commitment
    pub commitment_blinder: BlsScalar,
    /// Spend Commitment
    pub commitment_point: AffinePoint,
    /// Message Value within spend commitment
    pub spend_commitment_value: BlsScalar,
    /// Message Blinder within spend commitment
    pub spend_commitment_blinder: BlsScalar,
    /// Message spend Commitment
    pub spend_commitment: AffinePoint,
    /// Note Value within change commitment
    pub change_commitment_value: BlsScalar,
    /// Note Blinder within change commitment
    pub change_commitment_blinder: BlsScalar,
    /// Note change Commitment
    pub change_commitment: AffinePoint,
    /// Returns circuit size
    pub trim_size: usize,
    /// Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for WithdrawFromObfuscatedToContractCircuitOne {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<()> {
        let commitment_value = self
            .commitment_value;
        let commitment_blinder = self
            .commitment_blinder;
        let commitment_point = self
            .commitment_point;
        let spend_commitment_value = self
            .spend_commitment_value;
        let spend_commitment_blinder = self
            .spend_commitment_blinder;
        let spend_commitment = self
            .spend_commitment;
        let change_commitment_value = self
            .change_commitment_value;
        let change_commitment_blinder = self
            .change_commitment_blinder;
        let change_commitment = self
            .change_commitment;
        let pi = self.get_mut_pi_positions();

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
        let p2 =
            scalar_mul(composer, commitment_blind.var, GENERATOR_NUMS_EXTENDED);

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
        let p6 =
            scalar_mul(composer, change_blind.var, GENERATOR_NUMS_EXTENDED);

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

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'withdraw from obfuscated contract' transaction.
/// This is the second declared ciruit.
#[derive(Debug, Default, Clone)]
pub struct WithdrawFromObfuscatedToContractCircuitTwo {
    /// Spend Value within Pedersen commitment
    pub commitment_value: BlsScalar,
    /// Spend Blinder within Pedersen commitment
    pub commitment_blinder: BlsScalar,
    /// Spend Pedersen Commitment
    pub commitment_point: AffinePoint,
    /// Note Value within Pedersen commitment
    pub change_commitment_value: BlsScalar,
    /// Note Blinder within Pedersen commitment
    pub change_commitment_blinder: BlsScalar,
    /// Note Pedersen Commitment
    pub change_commitment: AffinePoint,
    /// Value to be sent
    pub value: BlsScalar,
    // Returns circuit size
    pub trim_size: usize,
    // Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for WithdrawFromObfuscatedToContractCircuitTwo {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<()> {
        let commitment_value = self
            .commitment_value;
        let commitment_blinder = self
            .commitment_blinder;
        let commitment_point = self
            .commitment_point;
        let change_commitment_value = self
            .change_commitment_value;
        let change_commitment_blinder = self
            .change_commitment_blinder;
        let change_commitment = self
            .change_commitment;
        let value = self
            .value;
        let pi = self.get_mut_pi_positions();

        // Create allocated scalars for private inputs
        let allocated_commitment_value =
            AllocatedScalar::allocate(composer, commitment_value);
        let allocated_commitment_blinder =
            AllocatedScalar::allocate(composer, commitment_blinder);
        let allocated_change_value =
            AllocatedScalar::allocate(composer, change_commitment_value);
        let allocated_change_blinder =
            AllocatedScalar::allocate(composer, change_commitment_blinder);

        // Allow adding of zero into circuit as a variable
        let zero =
            composer.add_witness_to_circuit_description(BlsScalar::zero());

        // Prove the knowledge of the commitment opening of the commitment of the input
        let p1 = scalar_mul(
            composer,
            allocated_commitment_value.var,
            GENERATOR_EXTENDED,
        );
        let p2 = scalar_mul(
            composer,
            allocated_commitment_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

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
        let p3 = scalar_mul(
            composer,
            allocated_change_value.var,
            GENERATOR_EXTENDED,
        );
        let p4 = scalar_mul(
            composer,
            allocated_change_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

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
        pi.push(PublicInput::BlsScalar(value, composer.circuit_size()));

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
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: commitment_point,
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: spend_commitment,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: change_commitment,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawOne")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
        ];

        circuit.verify_proof(
            &pub_params,
            &vk,
            b"ObfuscatedWithdrawOne",
            &proof,
            &pi,
        )
    }

    #[test]
    fn test_withdraw_from_obfuscated_to_contract_one_wrong_value() -> Result<()>
    {
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
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: commitment_point,
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: spend_commitment,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: change_commitment,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawOne")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
        ];

        assert!(circuit
            .verify_proof(
                &pub_params,
                &vk,
                b"ObfuscatedWithdrawOne",
                &proof,
                &pi
            )
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
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: commitment_point,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: change_commitment,
            value: value,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        circuit.verify_proof(
            &pub_params,
            &vk,
            b"ObfuscatedWithdrawTwo",
            &proof,
            &pi,
        )
    }

    #[test]
    fn test_withdraw_from_obfuscated_to_contract_two_wrong_value() -> Result<()>
    {
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
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: commitment_point,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: change_commitment,
            value: value,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        assert!(circuit
            .verify_proof(
                &pub_params,
                &vk,
                b"ObfuscatedWithdrawTwo",
                &proof,
                &pi
            )
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
            commitment_value: commitment_value.into(),
            commitment_blinder: commitment_blinder.into(),
            commitment_point: commitment_point,
            change_commitment_value: change_value.into(),
            change_commitment_blinder: change_blinder.into(),
            change_commitment: change_commitment,
            value: value,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdrawTwo")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_point, 0, 0),
            PublicInput::AffinePoint(change_commitment, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        assert!(circuit
            .verify_proof(
                &pub_params,
                &vk,
                b"ObfuscatedWithdrawTwo",
                &proof,
                &pi
            )
            .is_err());
        Ok(())
    }
}
