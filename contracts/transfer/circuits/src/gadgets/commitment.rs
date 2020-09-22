// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

// use phoenix_core::{Note, TransactionItem, ViewKey};

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
 AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED, ExtendedPoint,
};
use dusk_plonk::prelude::*;
use rand::*;
use plonk_gadgets::AllocatedScalar;


/// Prove knowledge of the value and blinding factor, which make up the value commitment.
/// This commitment gadget is using the pedersen commitments.
pub fn commitment(composer: &mut StandardComposer, value: AllocatedScalar, blinder: AllocatedScalar, pub_commit: AffinePoint) {
    
    let p1 = scalar_mul(composer, value.var, GENERATOR_EXTENDED);
    let p2 = scalar_mul(composer, blinder.var, GENERATOR_NUMS_EXTENDED);

    let commitment = p1.point().fast_add(composer, *p2.point());

    composer.assert_equal_public_point(commitment, pub_commit);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn commitment_gadget() {
        let value = JubJubScalar::from(100 as u64);
        let blinder = JubJubScalar::from(20000 as u64);

        let pc_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * value) 
                + &(GENERATOR_NUMS_EXTENDED * blinder)
        );

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 14, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 13).unwrap();
        let mut prover = Prover::new(b"test");

        let value = AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let blinder = AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(20000));

        commitment(prover.mut_cs(), value, blinder, pc_commitment);

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");

        let value = AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let blinder = AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(20000));

        commitment(verifier.mut_cs(), value, blinder, AffinePoint::from(pc_commitment));
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}
