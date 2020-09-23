// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

// Prove that the amount inputted equals the amount outputted
pub fn balance(
    composer: &mut StandardComposer,
    v_in: AllocatedScalar,
    v_out: AllocatedScalar,
    fee: AllocatedScalar,
) {
    let mut sum = composer.add_input(BlsScalar::zero());
    let zero = composer.add_witness_to_circuit_description(BlsScalar::zero());

    sum = composer.add(
        (BlsScalar::one(), sum),
        (BlsScalar::one(), v_in.var),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );

    sum = composer.add(
        (BlsScalar::one(), sum),
        (-BlsScalar::one(), v_out.var),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );

    composer.add_gate(
        sum,
        fee.var,
        zero,
        BlsScalar::one(),
        -BlsScalar::one(),
        BlsScalar::zero(),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn balance_gadget() -> Result<(), Error> {
        let v_in = 100 as u64;
        let v_out = 98 as u64;
        let fee = 2 as u64;

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");

        let v_in =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let v_out =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(98));
        let fee =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(2));

        balance(prover.mut_cs(), v_in, v_out, fee);

        let circuit = prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let v_in =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let v_out =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(98));
        let fee =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(2));

        balance(verifier.mut_cs(), v_in, v_out, fee);
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
