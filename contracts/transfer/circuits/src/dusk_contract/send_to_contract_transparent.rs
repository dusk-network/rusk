// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.


use dusk_plonk::prelude::*;
use crate::gadgets::{range::range, commitment::commitment};
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED 
};
use plonk_gadgets::AllocatedScalar;

pub fn send_to_contract_transparent(
    composer: &mut StandardComposer,
    commitment_crossover: AffinePoint, 
    commitment_crossover_value: AllocatedScalar,
    commitment_crossover_blinder: AllocatedScalar,
    value: AllocatedScalar, 
) {
    commitment(composer, commitment_crossover_value, commitment_crossover_blinder, commitment_crossover);
    
    range(composer, commitment_crossover_value, 64);

    composer.assert_equal(commitment_crossover_value.var, value.var);
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn test_send_to_contract_transparent() -> Result<(), Error> {

        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );


        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");

        let commitment_crossover_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
        let commitment_crossover_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let value = 
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));

        send_to_contract_transparent(
            prover.mut_cs(),
            commitment_crossover,
            commitment_crossover_value,
            commitment_crossover_blinder,
            value,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;


        let mut verifier = Verifier::new(b"test");

        let commitment_crossover_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        let commitment_crossover_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let value = 
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));

        send_to_contract_transparent(
            verifier.mut_cs(),
            commitment_crossover,
            commitment_crossover_value,
            commitment_crossover_blinder,
            value,
        );

        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
} 