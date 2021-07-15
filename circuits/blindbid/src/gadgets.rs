// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_blindbid::{Score, BID_HASHING_TYPE_FIELDS};
use dusk_bytes::Serializable;
use dusk_plonk::{constraint_system::ecc::Point as PlonkPoint, prelude::*};
use dusk_poseidon::sponge;
use plonk_gadgets::{
    AllocatedScalar, RangeGadgets::max_bound, ScalarGadgets::maybe_equal,
};

const SCALAR_FIELD_ORD_DIV_2_POW_128: BlsScalar = BlsScalar::from_raw([
    0x3339d80809a1d805,
    0x73eda753299d7d48,
    0x0000000000000000,
    0x0000000000000000,
]);

const MINUS_ONE_MOD_2_POW_128: BlsScalar = BlsScalar::from_raw([
    0xffffffff00000000,
    0x53bda402fffe5bfe,
    0x0000000000000000,
    0x0000000000000000,
]);

/// Hashes the internal Bid parameters using the Poseidon hash
/// function and the cannonical encoding for hashing returning a
/// Variable which contains the hash of the Bid.
pub(crate) fn preimage_gadget(
    composer: &mut StandardComposer,
    // TODO: We should switch to a different representation for this.
    // it can be a custom PoseidonCipherVariable structure or maybe
    // just a fixed len array of Variables.
    encrypted_data: (Variable, Variable),
    commitment: PlonkPoint,
    // (Pkr, R)
    stealth_addr: (PlonkPoint, PlonkPoint),
    hashed_secret: Variable,
    eligibility: Variable,
    expiration: Variable,
    pos: Variable,
) -> Variable {
    // This field represents the types of the inputs and has to be the same
    // as the default one.
    // It has been already checked that it's safe to unwrap here since the
    // value fits correctly in a `BlsScalar`.
    let type_fields = BlsScalar::from_bytes(&BID_HASHING_TYPE_FIELDS).unwrap();

    // Add to the composer the values required for the preimage.
    let messages: Vec<Variable> = vec![
        composer.add_input(type_fields),
        // Push cipher as scalars.
        encrypted_data.0,
        encrypted_data.1,
        // Push both JubJubAffine coordinates as a Scalar.
        *stealth_addr.0.x(),
        *stealth_addr.0.y(),
        // Push both JubJubAffine coordinates as a Scalar.
        *stealth_addr.1.x(),
        *stealth_addr.1.y(),
        hashed_secret,
        // Push both JubJubAffine coordinates as a Scalar.
        *commitment.x(),
        *commitment.y(),
        // Add elebility & expiration timestamps.
        eligibility,
        expiration,
        // Add position of the bid in the BidTree
        pos,
    ];

    // Perform the sponge_hash inside of the Constraint System
    sponge::gadget(composer, &messages)
}

/// Proves that a [`Score`] is correctly generated.
/// Prints the proving statements into the provided [`StandardComposer`].
///
/// Returns the value of the computed score as a [`Variable`].
pub(crate) fn score_correctness_gadget(
    composer: &mut StandardComposer,
    score: &Score,
    bid_value: AllocatedScalar,
    secret_k: AllocatedScalar,
    bid_tree_root: AllocatedScalar,
    consensus_round_seed: AllocatedScalar,
    latest_consensus_round: AllocatedScalar,
    latest_consensus_step: AllocatedScalar,
) -> Variable {
    // Allocate constant one & zero values.
    let one = composer.add_witness_to_circuit_description(BlsScalar::one());
    let zero = composer.add_witness_to_circuit_description(BlsScalar::zero());
    // Allocate Score fields needed for the gadget.
    let r1 = AllocatedScalar::allocate(composer, *score.r1());
    let r2 = AllocatedScalar::allocate(composer, *score.r2());
    let y = AllocatedScalar::allocate(composer, *score.y());
    let y_prime = AllocatedScalar::allocate(composer, *score.y_prime());
    let score_alloc_scalar =
        AllocatedScalar::allocate(composer, *score.value());
    let two_pow_128 = BlsScalar::from(2u64).pow(&[128, 0, 0, 0]);

    // 1. y = H(k||H(Bi)||sigma^s||k^t||k^s)
    let should_be_y = sponge::gadget(
        composer,
        &[
            secret_k.var,
            bid_tree_root.var,
            consensus_round_seed.var,
            latest_consensus_round.var,
            latest_consensus_step.var,
        ],
    );
    // Constrain the result of the hash to be equal to the Score y
    composer.assert_equal(should_be_y, y.var);

    // 2. Y = 2^128 * r1 + Y'
    composer.add_gate(
        y_prime.var,
        r1.var,
        y.var,
        BlsScalar::one(),
        two_pow_128,
        -BlsScalar::one(),
        BlsScalar::zero(),
        None,
    );
    // 3.(r1 < |Fr|/2^128 AND Y' < 2^128) OR (r1 = |Fr|/2^128 AND Y' < |Fr|
    // mod 2^128).
    //
    // 3.1. First op will be a complex rangeproof between r1 and the range
    // (Order of the Scalar Field / 2^128 (No modular division)) The result
    // should be 0 if the rangeproof holds.
    let first_cond = max_bound(composer, SCALAR_FIELD_ORD_DIV_2_POW_128, r1).0;

    // 3.2. Then we have a single Rangeproof between Y' being in the range
    // [0-2^128]
    let second_cond = max_bound(composer, two_pow_128, y_prime).0;
    // 3.3. Third, we have an equalty checking between r1 & the order of the
    // Scalar field divided (no modular division) by 2^128.
    // Since the gadget uses an `AllocatedScalar` here, we need to
    // previously constrain it's variable to a constant value: `the
    // order of the Scalar field divided (no modular division) by
    // 2^128` in this case. Then generate the `AllocatedScalar` and
    // call the gadget.
    let scalar_field_ord_div_2_128_variable = composer
        .add_witness_to_circuit_description(SCALAR_FIELD_ORD_DIV_2_POW_128);
    let scalar_field_ord_div_2_128 = AllocatedScalar {
        var: scalar_field_ord_div_2_128_variable,
        scalar: SCALAR_FIELD_ORD_DIV_2_POW_128,
    };
    // Now we can call the gadget with all the constraints applied to ensure
    // that the variable that represents 2^128
    let third_cond = maybe_equal(composer, scalar_field_ord_div_2_128, r1);
    // 3.4. Finally, constraints for y' checking it's between
    // [0, Order of the ScalarField mod 2^128].
    let fourth_cond = max_bound(composer, MINUS_ONE_MOD_2_POW_128, y_prime).0;
    // Apply the point 3 constraint.
    //(r1 < |Fr|/2^128 AND Y' < 2^128 +1)
    let left_assign = composer.mul(
        BlsScalar::one(),
        first_cond,
        second_cond,
        BlsScalar::zero(),
        None,
    );
    // (r1 = |Fr|/2^128 AND Y' < |Fr| mod 2^128)
    let right_assign = composer.mul(
        BlsScalar::one(),
        third_cond,
        fourth_cond,
        BlsScalar::zero(),
        None,
    );
    // left_assign XOR right_assign = 1
    // This is possible since condition 1. and 3. are mutually exclusive.
    // That means that if one is true, the other part of the
    // equation will be false (0). Therefore, we can apply a mul
    // gate since the inputs are boolean and both sides of the equal
    // can't be true, but both can be false, and this has to make
    // the proof fail. The following gate computes the XOR and
    // constraints the result to be equal to one.
    composer.add_gate(
        left_assign,
        right_assign,
        one,
        BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::zero(),
        -BlsScalar::one(),
        None,
    );

    // 4. r2 < Y'
    let r2_min_y_prime = composer.add(
        (BlsScalar::one(), r2.var),
        (-BlsScalar::one(), y_prime.var),
        BlsScalar::zero(),
        None,
    );
    let r2_min_y_prime_scalar = r2.scalar - y_prime.scalar;
    let r2_min_y_prime = AllocatedScalar {
        var: r2_min_y_prime,
        scalar: r2_min_y_prime_scalar,
    };

    // One indicates a failure here.
    let should_be_one = max_bound(
        composer,
        BlsScalar::from(2u64).pow(&[128, 0, 0, 0]),
        r2_min_y_prime,
    );

    // Check that the result of the range_proof is indeed 0 to assert it
    // passed.
    composer.constrain_to_constant(should_be_one.0, BlsScalar::one(), None);

    // 5. q < 2^120
    composer.range_gate(score_alloc_scalar.var, 120usize);
    // 5. q*Y' + r2 -d*2^128 = 0
    //
    // f * Y'
    let f_y_prime_prod = composer.mul(
        BlsScalar::one(),
        score_alloc_scalar.var,
        y_prime.var,
        BlsScalar::zero(),
        None,
    );
    // q*Y' + r2
    let left = composer.add(
        (BlsScalar::one(), f_y_prime_prod),
        (BlsScalar::one(), r2.var),
        BlsScalar::zero(),
        None,
    );
    // (q*Y' + r2) - v*2^128 = 0
    composer.add_gate(
        left,
        bid_value.var,
        zero,
        BlsScalar::one(),
        -two_pow_128,
        BlsScalar::zero(),
        BlsScalar::zero(),
        None,
    );

    score_alloc_scalar.var
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlindBidCircuitError;
    use dusk_blindbid::{Bid, V_RAW_MAX, V_RAW_MIN};
    use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;
    use phoenix_core::Message;
    use plonk_gadgets::AllocatedScalar;
    use rand::Rng;

    fn random_bid(secret: &JubJubScalar) -> (Bid, PublicSpendKey) {
        let mut rng = rand::thread_rng();
        let secret_k = BlsScalar::from(*secret);
        let psk = PublicSpendKey::from(SecretSpendKey::random(&mut rng));
        let value: u64 =
            (&mut rand::thread_rng()).gen_range(V_RAW_MIN..V_RAW_MAX);
        let eligibility_ts = u64::MAX;
        let expiration_ts = u64::MAX;

        (
            Bid::new(
                Message::new(&mut rng, &secret, &psk, value),
                secret_k,
                psk.gen_stealth_address(&secret),
                eligibility_ts,
                expiration_ts,
            ),
            psk,
        )
    }

    fn allocate_fields(
        composer: &mut StandardComposer,
        value: JubJubScalar,
        secret_k: BlsScalar,
        bid_tree_root: BlsScalar,
        consensus_round_seed: BlsScalar,
        latest_consensus_round: BlsScalar,
        latest_consensus_step: BlsScalar,
    ) -> (
        AllocatedScalar,
        AllocatedScalar,
        AllocatedScalar,
        AllocatedScalar,
        AllocatedScalar,
        AllocatedScalar,
    ) {
        let value = AllocatedScalar::allocate(composer, value.into());

        let secret_k = AllocatedScalar::allocate(composer, secret_k);
        let bid_tree_root = AllocatedScalar::allocate(composer, bid_tree_root);
        let consensus_round_seed =
            AllocatedScalar::allocate(composer, consensus_round_seed);
        let latest_consensus_round =
            AllocatedScalar::allocate(composer, latest_consensus_round);
        let latest_consensus_step =
            AllocatedScalar::allocate(composer, latest_consensus_step);
        (
            value,
            secret_k,
            bid_tree_root,
            consensus_round_seed,
            latest_consensus_round,
            latest_consensus_step,
        )
    }

    #[test]
    fn bid_preimage_gadget() -> Result<(), BlindBidCircuitError> {
        // Generate Composer & Public Parameters
        let pub_params = unsafe {
            PublicParameters::from_slice_unchecked(
                rusk_profile::get_common_reference_string()
                    .expect("Failed to fetch CRS from rusk_profile")
                    .as_slice(),
            )
        };
        let (ck, vk) = pub_params.trim(1 << 13)?;

        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let (bid, psk) = random_bid(&secret);

        let circuit = |composer: &mut StandardComposer, bid: &Bid| {
            // Allocate Bid-internal fields
            let bid_hashed_secret =
                AllocatedScalar::allocate(composer, *bid.hashed_secret());
            let bid_cipher = (
                composer.add_input(bid.encrypted_data()[0]),
                composer.add_input(bid.encrypted_data()[1]),
            );
            let bid_commitment =
                composer.add_affine(JubJubAffine::from(bid.commitment()));
            let bid_stealth_addr = (
                composer
                    .add_affine(bid.stealth_address().pk_r().as_ref().into()),
                composer.add_affine(bid.stealth_address().R().into()),
            );
            let eligibility = AllocatedScalar::allocate(
                composer,
                BlsScalar::from(*bid.eligibility()),
            );
            let expiration = AllocatedScalar::allocate(
                composer,
                BlsScalar::from(*bid.expiration()),
            );
            let pos = AllocatedScalar::allocate(
                composer,
                BlsScalar::from(*bid.pos()),
            );
            let bid_hash = preimage_gadget(
                composer,
                bid_cipher,
                bid_commitment,
                bid_stealth_addr,
                bid_hashed_secret.var,
                eligibility.var,
                expiration.var,
                pos.var,
            );

            // Constraint the hash to be equal to the real one
            let storage_bid = bid.hash();
            composer.constrain_to_constant(
                bid_hash,
                BlsScalar::zero(),
                Some(-storage_bid),
            );
        };
        // Proving
        let mut prover = Prover::new(b"testing");
        circuit(prover.mut_cs(), &bid);
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        // Verification
        let mut verifier = Verifier::new(b"testing");
        circuit(verifier.mut_cs(), &bid);
        verifier.preprocess(&ck)?;
        let pi = verifier.mut_cs().construct_dense_pi_vec();
        Ok(verifier.verify(&proof, &vk, &pi)?)
    }

    #[test]
    fn correct_score_gen_proof() -> Result<(), BlindBidCircuitError> {
        // Generate Composer & Public Parameters
        let pub_params = unsafe {
            PublicParameters::from_slice_unchecked(
                rusk_profile::get_common_reference_string()
                    .expect("Failed to fetch CRS from rusk_profile")
                    .as_slice(),
            )
        };
        let (ck, vk) = pub_params.trim(1 << 16)?;

        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let (bid, psk) = random_bid(&secret);
        let (value, _) = bid
            .decrypt_data(&secret.into(), &psk)
            .expect("Decryption error");

        // Generate fields for the Bid & required by the compute_score
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid_tree_root = BlsScalar::random(&mut rand::thread_rng());
        let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());
        // Set latest consensus round as the max value so the score gen does not
        // fail for that but for the proof verification error if that's
        // the case
        let latest_consensus_round = 25519u64;
        let latest_consensus_step = 2u64;

        // Edit score fields which should make the test fail
        let score = Score::compute(
            &bid,
            &secret.into(),
            &psk,
            secret_k,
            bid_tree_root,
            consensus_round_seed,
            latest_consensus_round,
            latest_consensus_step,
        )?;

        // Proving
        let mut prover = Prover::new(b"testing");

        // Allocate values
        let (
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        ) = allocate_fields(
            prover.mut_cs(),
            value,
            secret_k,
            bid_tree_root,
            BlsScalar::from(consensus_round_seed),
            BlsScalar::from(latest_consensus_round),
            BlsScalar::from(latest_consensus_step),
        );

        score_correctness_gadget(
            prover.mut_cs(),
            &score,
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        // Verification
        let mut verifier = Verifier::new(b"testing");
        // Allocate values
        let (
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        ) = allocate_fields(
            verifier.mut_cs(),
            value,
            secret_k,
            bid_tree_root,
            BlsScalar::from(consensus_round_seed),
            BlsScalar::from(latest_consensus_round),
            BlsScalar::from(latest_consensus_step),
        );

        score_correctness_gadget(
            verifier.mut_cs(),
            &score,
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        );

        verifier.preprocess(&ck)?;
        Ok(verifier.verify(&proof, &vk, &vec![BlsScalar::zero()])?)
    }

    #[test]
    fn incorrect_score_gen_proof() -> Result<(), BlindBidCircuitError> {
        // Generate Composer & Public Parameters
        let pub_params = unsafe {
            PublicParameters::from_slice_unchecked(
                rusk_profile::get_common_reference_string()
                    .expect("Failed to fetch CRS from rusk_profile")
                    .as_slice(),
            )
        };
        let (ck, vk) = pub_params.trim(1 << 16)?;

        // Generate a correct Bid
        let secret = JubJubScalar::random(&mut rand::thread_rng());
        let (bid, psk) = random_bid(&secret);
        let (value, _) =
            bid.decrypt_data(&secret, &psk).expect("Decryption Error");

        // Generate fields for the Bid & required by the compute_score
        let secret_k = BlsScalar::random(&mut rand::thread_rng());
        let bid_tree_root = BlsScalar::random(&mut rand::thread_rng());
        let consensus_round_seed = BlsScalar::random(&mut rand::thread_rng());
        // Set the timestamps to the maximum possible value so the generation of
        // the score does not fail for that reason but for the proof
        // verification.
        let latest_consensus_round = 25519u64;
        let latest_consensus_step = 2u64;

        // The only way to generate a random `Score` is to unsafely do so.
        let score = unsafe {
            std::mem::transmute_copy(&[
                BlsScalar::from(5686536568u64),
                BlsScalar::from(5686536568u64),
                BlsScalar::from(5686536568u64),
                BlsScalar::from(5686536568u64),
                BlsScalar::from(5686536568u64),
            ])
        };

        // Proving
        let mut prover = Prover::new(b"testing");
        // Allocate values
        let (
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        ) = allocate_fields(
            prover.mut_cs(),
            value,
            secret_k,
            bid_tree_root,
            BlsScalar::from(consensus_round_seed),
            BlsScalar::from(latest_consensus_round),
            BlsScalar::from(latest_consensus_step),
        );

        score_correctness_gadget(
            prover.mut_cs(),
            &score,
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        // Verification
        let mut verifier = Verifier::new(b"testing");
        // Allocate values
        let (
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        ) = allocate_fields(
            verifier.mut_cs(),
            value,
            secret_k,
            bid_tree_root,
            BlsScalar::from(consensus_round_seed),
            BlsScalar::from(latest_consensus_round),
            BlsScalar::from(latest_consensus_step),
        );

        score_correctness_gadget(
            verifier.mut_cs(),
            &score,
            alloc_value,
            alloc_secret_k,
            alloc_bid_tree_root,
            alloc_consensus_round_seed,
            alloc_latest_consensus_round,
            alloc_latest_consensus_step,
        );

        verifier.preprocess(&ck)?;
        assert!(verifier
            .verify(&proof, &vk, &vec![BlsScalar::zero()])
            .is_err());

        Ok(())
    }
}
