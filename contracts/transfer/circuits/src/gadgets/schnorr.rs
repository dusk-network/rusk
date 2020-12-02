// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul as fixed_base_scalar_mul;
use dusk_plonk::constraint_system::ecc::scalar_mul::variable_base::variable_base_scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;

/// Utilises a Schnorr signature scheme,
/// to prove the knowledge of a discrete
/// log for a given public key.
#[allow(non_snake_case)]
pub fn schnorr_gadget_one_key(
    composer: &mut StandardComposer,
    signature: AllocatedScalar,
    R: PlonkPoint,
    pk: PlonkPoint,
    message: AllocatedScalar,
) {
    let h = sponge_hash_gadget(composer, &[message.var]);
    let c = sponge_hash_gadget(composer, &[*R.x(), *R.y(), h]);
    let b = composer.add_witness_to_circuit_description(BlsScalar::zero());

    let challenge = composer.xor_gate(c, b, 250);

    let sig =
        fixed_base_scalar_mul(composer, signature.var, GENERATOR_EXTENDED);
    let p = variable_base_scalar_mul(composer, challenge, pk);

    let add = sig.point().add(composer, *p.point());
    composer.assert_equal_point(add, R);
}

/// Utilises a Schnorr signature scheme,
/// to prove the knowledge of the discrete
/// log for given keys in a public key pair.
/// Also verifying that both keys share the
/// same discrete log.
#[allow(non_snake_case)]
pub fn schnorr_gadget_two_keys(
    composer: &mut StandardComposer,
    signature: AllocatedScalar,
    R: PlonkPoint,
    R_prime: PlonkPoint,
    pk: PlonkPoint,
    pk_prime: PlonkPoint,
    message: AllocatedScalar,
) {
    let h = sponge_hash_gadget(composer, &[message.var]);
    let c = sponge_hash_gadget(
        composer,
        &[*R.x(), *R.y(), *R_prime.x(), *R_prime.y(), h],
    );

    let b = composer.add_witness_to_circuit_description(BlsScalar::zero());

    let challenge = composer.xor_gate(c, b, 250);
    let sig_1 =
        fixed_base_scalar_mul(composer, signature.var, GENERATOR_EXTENDED);
    let sig_2 =
        fixed_base_scalar_mul(composer, signature.var, GENERATOR_NUMS_EXTENDED);
    let pub_1 = variable_base_scalar_mul(composer, challenge, pk);
    let pub_2 = variable_base_scalar_mul(composer, challenge, pk_prime);

    let add_1 = sig_1.point().add(composer, *pub_1.point());
    composer.assert_equal_point(add_1, R);
    let add_2 = sig_2.point().add(composer, *pub_2.point());
    composer.assert_equal_point(add_2, R_prime);
}

#[cfg(test)]
mod schnorr_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::jubjub::JubJubAffine;
    use poseidon252::sponge::hash;

    #[test]
    #[allow(non_snake_case)]
    fn schnorr_two_keys() -> Result<(), Error> {
        // Setup
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let message = BlsScalar::random(&mut rand::thread_rng());
        let pk = JubJubAffine::from(GENERATOR_EXTENDED * sk);
        let pk_prime = JubJubAffine::from(GENERATOR_NUMS_EXTENDED * sk);

        // Signing
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = JubJubAffine::from(GENERATOR_EXTENDED * r);
        let R_prime = JubJubAffine::from(GENERATOR_NUMS_EXTENDED * r);
        let h = hash(&[message]);
        let c_hash =
            hash(&[R.get_x(), R.get_y(), R_prime.get_x(), R_prime.get_y(), h]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let mut prover = Prover::new(b"test");

        let sig_a = AllocatedScalar::allocate(prover.mut_cs(), U.into());
        let R_p = PlonkPoint::from_private_affine(prover.mut_cs(), R);
        let R_prime_p =
            PlonkPoint::from_private_affine(prover.mut_cs(), R_prime);
        let pk_p = PlonkPoint::from_private_affine(prover.mut_cs(), pk);
        let pk_prime_p =
            PlonkPoint::from_private_affine(prover.mut_cs(), pk_prime);
        let message_a = AllocatedScalar::allocate(prover.mut_cs(), message);

        schnorr_gadget_two_keys(
            prover.mut_cs(),
            sig_a,
            R_p,
            R_prime_p,
            pk_p,
            pk_prime_p,
            message_a,
        );
        let prover_pi = prover.mut_cs().public_inputs.clone();
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");
        let sig = AllocatedScalar::allocate(verifier.mut_cs(), U.into());
        let R = PlonkPoint::from_private_affine(verifier.mut_cs(), R);
        let R_prime =
            PlonkPoint::from_private_affine(verifier.mut_cs(), R_prime);
        let pk = PlonkPoint::from_private_affine(verifier.mut_cs(), pk);
        let pk_prime =
            PlonkPoint::from_private_affine(verifier.mut_cs(), pk_prime);
        let message = AllocatedScalar::allocate(verifier.mut_cs(), message);

        schnorr_gadget_two_keys(
            verifier.mut_cs(),
            sig,
            R,
            R_prime,
            pk,
            pk_prime,
            message,
        );
        verifier.preprocess(&ck)?;
        verifier.verify(&proof, &vk, &prover_pi)
    }

    #[test]
    #[allow(non_snake_case)]
    fn schnorr_one_key() -> Result<(), Error> {
        // Setup
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let message = BlsScalar::random(&mut rand::thread_rng());
        let pk = JubJubAffine::from(GENERATOR_EXTENDED * sk);

        // Signing
        let r = JubJubScalar::random(&mut rand::thread_rng());
        let R = JubJubAffine::from(GENERATOR_EXTENDED * r);
        let h = hash(&[message]);
        let c_hash = hash(&[R.get_x(), R.get_y(), h]);
        let c_hash = c_hash & BlsScalar::pow_of_2(250).sub(&BlsScalar::one());
        let c = JubJubScalar::from_bytes(&c_hash.to_bytes()).unwrap();
        let U = r - (c * sk);

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let mut prover = Prover::new(b"test");

        let sig_a = AllocatedScalar::allocate(prover.mut_cs(), U.into());
        let R_p = PlonkPoint::from_private_affine(prover.mut_cs(), R);
        let pk_p = PlonkPoint::from_private_affine(prover.mut_cs(), pk);
        let message_a = AllocatedScalar::allocate(prover.mut_cs(), message);

        schnorr_gadget_one_key(prover.mut_cs(), sig_a, R_p, pk_p, message_a);
        let prover_pi = prover.mut_cs().public_inputs.clone();
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");
        let sig = AllocatedScalar::allocate(verifier.mut_cs(), U.into());
        let R = PlonkPoint::from_private_affine(verifier.mut_cs(), R);
        let pk = PlonkPoint::from_private_affine(verifier.mut_cs(), pk);
        let message = AllocatedScalar::allocate(verifier.mut_cs(), message);

        schnorr_gadget_one_key(verifier.mut_cs(), sig, R, pk, message);
        verifier.preprocess(&ck)?;
        verifier.verify(&proof, &vk, &prover_pi)
    }
}
