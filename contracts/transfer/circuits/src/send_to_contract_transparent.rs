// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use anyhow::Result;
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::JubJubExtended;
use dusk_plonk::prelude::*;
use schnorr::single_key::Signature;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct SendToContractTransparentCircuit {
    pi_positions: Vec<PublicInput>,

    blinding_factor: JubJubScalar,
    signature: Signature,

    // Public data
    value_commitment: JubJubExtended,
    pk: JubJubExtended,
    value: BlsScalar,
}

impl SendToContractTransparentCircuit {
    pub fn new(
        value_commitment: JubJubExtended,
        pk: JubJubExtended,
        value: u64,
        blinding_factor: JubJubScalar,
        signature: Signature,
    ) -> Self {
        Self {
            pi_positions: vec![],
            blinding_factor,
            signature,
            value: BlsScalar::from(value),
            value_commitment,
            pk,
        }
    }
}

impl Circuit<'_> for SendToContractTransparentCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let mut pi = vec![];

        // 1. Prove the knowledge of the commitment opening of the
        // commitment
        let value = composer.add_input(self.value);

        let blinding_factor = self.blinding_factor.into();
        let blinding_factor = composer.add_input(blinding_factor);

        let value_commitment_p =
            gadgets::commitment(composer, value, blinding_factor);

        let value_commitment = self.value_commitment.into();
        pi.push(PublicInput::AffinePoint(
            value_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        let value_commitment = value_commitment_p;

        // 2. Prove that the value of the opening of the commitment
        // of the Crossover is within range
        gadgets::range(composer, value);

        // 3. Verify the Schnorr proof corresponding to the commitment
        // public key
        let pk = self.pk.into();
        pi.push(PublicInput::AffinePoint(
            pk,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        let pk = Point::from_public_affine(composer, pk);

        let r = Point::from_private_affine(composer, self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        gadgets::point_signature(composer, value_commitment, pk, r, u);

        // 4. Prove that v_c - v = 0
        pi.push(PublicInput::BlsScalar(self.value, composer.circuit_size()));
        composer.constrain_to_constant(value, BlsScalar::zero(), -self.value);

        self.get_mut_pi_positions().extend_from_slice(pi.as_slice());

        Ok(())
    }

    /// Returns the size at which we trim the `PublicParameters`
    /// to compile the circuit or perform proving/verification
    /// actions.
    fn get_trim_size(&self) -> usize {
        1 << 13
    }

    fn set_trim_size(&mut self, _size: usize) {
        // N/A, fixed size circuit
    }

    /// Return a mutable reference to the Public Inputs storage of the
    /// circuit.
    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    /// Return a reference to the Public Inputs storage of the circuit.
    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }
}
