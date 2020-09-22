// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use phoenix_core::note::Note;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use poseidon252::sponge::sponge::{sponge_hash_gadget, sponge_hash};
use dusk_bls12_381::Scalar;
use plonk_gadgets::AllocatedScalar;
use dusk_pki::Ownable;

pub fn nullifier(composer: &mut StandardComposer, pos: AllocatedScalar, sk: AllocatedScalar, nullifier: AllocatedScalar) {
    let zero = composer.add_witness_to_circuit_description(Scalar::zero());
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
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;

    #[test]
    fn nullifier_gadget() {
        let pos_scalar = BlsScalar::from(1);
        let sk_scalar = BlsScalar::from(100);
        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 14, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 13).unwrap();
        let mut prover = Prover::new(b"test");

        let pos = AllocatedScalar::allocate(prover.mut_cs(), pos_scalar);
        let sk = AllocatedScalar::allocate(prover.mut_cs(), sk_scalar);
        let nul_scalar = sponge_hash(&[sk_scalar, pos_scalar]);
        let nul = AllocatedScalar::allocate(prover.mut_cs(), nul_scalar);

        nullifier(prover.mut_cs(), pos, sk, nul);
        prover.mut_cs().add_dummy_constraints();

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");

        let pos = AllocatedScalar::allocate(verifier.mut_cs(), pos_scalar);
        let sk = AllocatedScalar::allocate(verifier.mut_cs(), sk_scalar);
        let nul = AllocatedScalar::allocate(verifier.mut_cs(), nul_scalar);

        nullifier(verifier.mut_cs(), pos, sk, nul);
        verifier.mut_cs().add_dummy_constraints();
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}
