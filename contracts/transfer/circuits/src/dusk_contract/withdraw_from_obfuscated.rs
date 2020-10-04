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
pub struct WithdrawFromContractObfuscatedCircuit {
    /// Spend Value within Pedersen commitment
    pub spend_commitment_value: BlsScalar,
    /// Spend Blinder within Pedersen commitment
    pub spend_commitment_blinder: BlsScalar,
    /// Spend Pedersen Commitment
    pub spend_commitment: AffinePoint,
    /// Message Value within Pedersen commitment
    pub message_commitment_value: BlsScalar,
    /// Message Blinder within Pedersen commitment
    pub message_commitment_blinder: BlsScalar,
    /// Message Pedersen Commitment
    pub message_commitment: AffinePoint,
    /// Note Value within Pedersen commitment
    pub note_commitment_value: BlsScalar,
    /// Note Blinder within Pedersen commitment
    pub note_commitment_blinder: BlsScalar,
    /// Note Pedersen Commitment
    pub note_commitment: AffinePoint,
    /// Returns circuit size
    pub trim_size: usize,
    /// Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for WithdrawFromContractObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<()> {
        let spend_commitment_value = self
            .spend_commitment_value;
        let spend_commitment_blinder = self
            .spend_commitment_blinder;
        let spend_commitment = self
            .spend_commitment;
        let message_commitment_value = self
            .message_commitment_value;
        let message_commitment_blinder = self
            .message_commitment_blinder;
        let message_commitment: AffinePoint = self
            .message_commitment;
        let note_commitment_value = self
            .note_commitment_value;
        let note_commitment_blinder = self
            .note_commitment_blinder;
        let note_commitment = self
            .note_commitment;
        let pi = self.get_mut_pi_positions();

        // Create allocated scalars for private inputs
        let spend_value =
            AllocatedScalar::allocate(composer, spend_commitment_value);
        let spend_blind =
            AllocatedScalar::allocate(composer, spend_commitment_blinder);
        let message_value =
            AllocatedScalar::allocate(composer, message_commitment_value);
        let message_blind =
            AllocatedScalar::allocate(composer, message_commitment_blinder);
        let note_value =
            AllocatedScalar::allocate(composer, note_commitment_value);
        let note_blind =
            AllocatedScalar::allocate(composer, note_commitment_blinder);

        // Prove the knowledge of the commitment opening of the spend commitment of the input
        let p1 = scalar_mul(composer, spend_value.var, GENERATOR_EXTENDED);
        let p2 = scalar_mul(composer, spend_blind.var, GENERATOR_NUMS_EXTENDED);

        let commitment = p1.point().fast_add(composer, *p2.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            spend_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, spend_commitment);

        range(composer, spend_value, 64);

        let p3 = scalar_mul(composer, message_value.var, GENERATOR_EXTENDED);
        let p4 =
            scalar_mul(composer, message_blind.var, GENERATOR_NUMS_EXTENDED);

        let commitment2 = p3.point().fast_add(composer, *p4.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            message_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment2, message_commitment);

        // Prove that the value of the opening of the message commitment of the input is within range
        range(composer, message_value, 64);

        let p5 = scalar_mul(composer, note_value.var, GENERATOR_EXTENDED);
        let p6 = scalar_mul(composer, note_blind.var, GENERATOR_NUMS_EXTENDED);

        let commitment3 = p5.point().fast_add(composer, *p6.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            note_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment3, note_commitment);

        // Prove that the value of the opening of the note commitment of the input is within range
        range(composer, note_value, 64);

        // Constrain the value inputs to satisfy: v_spend - v_message - v_note = 0
        composer.add_gate(
            message_value.var,
            note_value.var,
            spend_value.var,
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;

    #[test]
    fn test_withdraw_from_obfuscated() -> Result<()> {
        // Define and create spend commitment values
        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(100 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        // Define and create message commitment values
        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(200 as u64);
        let message_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * message_value)
                + &(GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        // Define and create note commitment values
        let note_value = JubJubScalar::from(100 as u64);
        let note_blinder = JubJubScalar::from(300 as u64);
        let note_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * note_value)
                + &(GENERATOR_NUMS_EXTENDED * note_blinder),
        );

        // Build circuit structure
        let mut circuit = WithdrawFromContractObfuscatedCircuit {
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: spend_commitment,
            message_commitment_value: message_value.into(),
            message_commitment_blinder: message_blinder.into(),
            message_commitment: message_commitment,
            note_commitment_value: note_value.into(),
            note_commitment_blinder: note_blinder.into(),
            note_commitment: note_commitment,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdraw")?;

        let pi = vec![
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(message_commitment, 0, 0),
            PublicInput::AffinePoint(note_commitment, 0, 0),
        ];

        circuit.verify_proof(
            &pub_params,
            &vk,
            b"ObfuscatedWithdraw",
            &proof,
            &pi,
        )
    }

    #[test]
    fn test_withdraw_from_obfuscated_wrong_value() -> Result<()> {
        // Define and create spend commitment values
        let spend_value = JubJubScalar::from(200 as u64);
        let spend_blinder = JubJubScalar::from(100 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        // Define and create message commitment values
        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(200 as u64);
        let message_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * message_value)
                + &(GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        // Define and create note commitment values
        let note_value = JubJubScalar::from(100 as u64);
        let note_blinder = JubJubScalar::from(300 as u64);
        let note_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * note_value)
                + &(GENERATOR_NUMS_EXTENDED * note_blinder),
        );

        // Build circuit structure
        let mut circuit = WithdrawFromContractObfuscatedCircuit {
            spend_commitment_value: spend_value.into(),
            spend_commitment_blinder: spend_blinder.into(),
            spend_commitment: spend_commitment,
            message_commitment_value: message_value.into(),
            message_commitment_blinder: message_blinder.into(),
            message_commitment: message_commitment,
            note_commitment_value: note_value.into(),
            note_commitment_blinder: note_blinder.into(),
            note_commitment: note_commitment,
            trim_size: 1 << 12,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 13, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof =
            circuit.gen_proof(&pub_params, &pk, b"ObfuscatedWithdraw")?;

        let pi = vec![
            PublicInput::AffinePoint(spend_commitment, 0, 0),
            PublicInput::AffinePoint(message_commitment, 0, 0),
            PublicInput::AffinePoint(note_commitment, 0, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedWithdraw", &proof, &pi)
            .is_err());
        Ok(())
    }
}
