// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::Ownable;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use phoenix_core::note::Note;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::{sponge_hash, sponge_hash_gadget};

pub fn nullifier(
    composer: &mut StandardComposer,
    pos: AllocatedScalar,
    sk: AllocatedScalar,
    nullifier: AllocatedScalar,
) {
    let zero = composer.add_witness_to_circuit_description(BlsScalar::zero());
    let output = sponge_hash_gadget(composer, &[sk.var, pos.var]);

    composer.add_gate(
        output,
        zero,
        zero,
        -BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::zero(),
        nullifier.scalar,
    );
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;

    #[test]
    fn nullifier_gadget() -> Result<(), Error> {
        let pos_scalar = BlsScalar::from(1);
        let sk_scalar = BlsScalar::from(100);
        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let mut prover = Prover::new(b"test");

        let pos = AllocatedScalar::allocate(prover.mut_cs(), pos_scalar);
        let sk = AllocatedScalar::allocate(prover.mut_cs(), sk_scalar);
        let nul_scalar = sponge_hash(&[sk_scalar, pos_scalar]);
        let nul = AllocatedScalar::allocate(prover.mut_cs(), nul_scalar);

        nullifier(prover.mut_cs(), pos, sk, nul);

        let circuit = prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let pos = AllocatedScalar::allocate(verifier.mut_cs(), pos_scalar);
        let sk = AllocatedScalar::allocate(verifier.mut_cs(), sk_scalar);
        let nul = AllocatedScalar::allocate(verifier.mut_cs(), nul_scalar);

        nullifier(verifier.mut_cs(), pos, sk, nul);
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
