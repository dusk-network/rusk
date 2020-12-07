// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets::{range::range, schnorr::schnorr_gadget_one_key};
use anyhow::Result;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{
    JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Default, Clone)]
pub struct SendToContractTransparentCircuit {
    /// Value within Pedersen commitment
    pub commitment_value: BlsScalar,
    /// Blinder within Pedersen commitment
    pub blinder: BlsScalar,
    /// Pedersen Commitment
    pub commitment: JubJubAffine,
    /// Value to be sent
    pub value: BlsScalar,
    /// Public key
    pub pk: JubJubAffine,
    /// Schnorr signature
    pub schnorr_sig: JubJubScalar,
    /// Schnorr R
    pub schnorr_r: JubJubAffine,
    /// Schnorr PK
    pub schnorr_pk: JubJubAffine,
    /// Schnorr message
    pub schnorr_message: BlsScalar,
    /// Returns circuit size
    pub trim_size: usize,
    /// Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for SendToContractTransparentCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let commitment_crossover = self.commitment;
        let commitment_crossover_value = self.commitment_value;
        let commitment_crossover_blinder = self.blinder;
        let schnorr_sig = self.schnorr_sig;
        let schnorr_r = self.schnorr_r;
        let schnorr_pk = self.schnorr_pk;
        let schnorr_message = self.schnorr_message;

        let value = self.value;
        let pk = self.pk;
        let pi = self.get_mut_pi_positions();

        // Create allocated scalars for private inputs
        let allocated_commitment_crossover_value =
            AllocatedScalar::allocate(composer, commitment_crossover_value);
        let allocated_commitment_crossover_blinder =
            AllocatedScalar::allocate(composer, commitment_crossover_blinder);
        let schnorr_sig =
            AllocatedScalar::allocate(composer, schnorr_sig.into());
        let schnorr_r = PlonkPoint::from_private_affine(composer, schnorr_r);
        let schnorr_pk = PlonkPoint::from_private_affine(composer, schnorr_pk);
        let schnorr_message =
            AllocatedScalar::allocate(composer, schnorr_message);

        // Prove the knowledge of the commitment opening of the commitment of
        // the crossover in the input
        let p1 = scalar_mul(
            composer,
            allocated_commitment_crossover_value.var,
            GENERATOR_EXTENDED,
        );
        let p2 = scalar_mul(
            composer,
            allocated_commitment_crossover_blinder.var,
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

        // Prove that the value of the opening of the commitment of the input is
        // within range
        range(composer, allocated_commitment_crossover_value, 64);

        //Assert the given private and public pk inputs are equal
        pi.push(PublicInput::AffinePoint(
            pk,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        composer.assert_equal_public_point(schnorr_pk, pk);

        // Verify the Schnorr signature
        schnorr_gadget_one_key(
            composer,
            schnorr_sig,
            schnorr_r,
            schnorr_pk,
            schnorr_message,
        );

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::BlsScalar(value, composer.circuit_size()));

        // Constrains the crossover value to equal the PI value
        composer.constrain_to_constant(
            allocated_commitment_crossover_value.var,
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

    /// /// Return a mutable reference to the Public Inputs storage of the
    /// circuit.
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
    use poseidon252::sponge::sponge::sponge_hash;

    fn schnorr_sign(
        sk: JubJubScalar,
        message: BlsScalar,
    ) -> (JubJubScalar, JubJubAffine, JubJubAffine) {
        let pk = JubJubAffine::from(GENERATOR_EXTENDED * sk);
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = JubJubAffine::from(GENERATOR_EXTENDED * r);
        let h = sponge_hash(&[message]);
        let c_hash = sponge_hash(&[R.get_x(), R.get_y(), h]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);
        (U, R, pk)
    }

    #[test]
    fn test_send_to_contract_transparent() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = JubJubAffine::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(300);

        let message = BlsScalar::random(&mut rand::thread_rng());
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sig = schnorr_sign(sk, message);
        let public_key = JubJubAffine::from(GENERATOR_EXTENDED * sk);

        // Build circuit structure
        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: commitment_crossover_value.into(),
            blinder: commitment_crossover_blinder.into(),
            commitment: commitment_crossover,
            value: value,
            pk: public_key,
            schnorr_sig: sig.0,
            schnorr_r: sig.1,
            schnorr_pk: sig.2,
            schnorr_message: message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"TransparentSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(public_key, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        circuit.verify_proof(&pub_params, &vk, b"TransparentSend", &proof, &pi)
    }

    #[test]
    fn test_send_to_contract_transparent_wrong_value() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(500 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = JubJubAffine::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(300);

        let message = BlsScalar::random(&mut rand::thread_rng());
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sig = schnorr_sign(sk, message);
        let public_key = JubJubAffine::from(GENERATOR_EXTENDED * sk);

        // Build circuit structure
        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: commitment_crossover_value.into(),
            blinder: commitment_crossover_blinder.into(),
            commitment: commitment_crossover,
            value: value,
            pk: public_key,
            schnorr_sig: sig.0,
            schnorr_r: sig.1,
            schnorr_pk: sig.2,
            schnorr_message: message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"TransparentSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(public_key, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"TransparentSend", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_send_to_contract_transparent_wrong_pi() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = JubJubAffine::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        // Declare value for PI input
        let value = BlsScalar::from(100);

        let message = BlsScalar::random(&mut rand::thread_rng());
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sig = schnorr_sign(sk, message);
        let public_key = JubJubAffine::from(GENERATOR_EXTENDED * sk);

        // Build circuit structure
        let mut circuit = SendToContractTransparentCircuit {
            commitment_value: commitment_crossover_value.into(),
            blinder: commitment_crossover_blinder.into(),
            commitment: commitment_crossover,
            value: value,
            pk: public_key,
            schnorr_sig: sig.0,
            schnorr_r: sig.1,
            schnorr_pk: sig.2,
            schnorr_message: message,
            trim_size: 1 << 13,
            pi_positions: vec![],
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"TransparentSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(public_key, 0, 0),
            PublicInput::BlsScalar(value, 0),
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"TransparentSend", &proof, &pi)
            .is_err());
        Ok(())
    }
}
