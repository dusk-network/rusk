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

pub fn withdraw_from_obfuscated_to_contract_one(
    composer: &mut StandardComposer,
    commitment_one: AffinePoint,
    commitment_one_value: AllocatedScalar,
    commitment_one_blinder: AllocatedScalar,
    message_spend_commitment: AffinePoint,
    message_spend_commitment_value: AllocatedScalar, 
    message_spend_commitment_blinder: AllocatedScalar,
    message_change_commitment: AffinePoint,
    message_change_commitment_value: AllocatedScalar,
    message_change_commitment_blinder: AllocatedScalar,
) {

    commitment(composer, commitment_one_value, commitment_one_blinder, commitment_one);
    commitment(composer, message_spend_commitment_value, message_spend_commitment_blinder, message_spend_commitment);
    commitment(composer, message_change_commitment_value, message_change_commitment_blinder, message_change_commitment);

    range(composer, commitment_one_value, 64);
    range(composer, message_spend_commitment_value, 64);
    range(composer, message_change_commitment_value, 64);

    composer.add_gate(
        message_spend_commitment_value.var,
        message_change_commitment_value.var,
        commitment_one_value.var,
        BlsScalar::one(),
        BlsScalar::one(),
        -BlsScalar::one(),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );
}

pub fn withdraw_from_obfuscated_to_contract_two(
    composer: &mut StandardComposer,
    commitment_two: AffinePoint,
    commitment_two_value: AllocatedScalar,
    commitment_two_blinder: AllocatedScalar,
    change_message_commitment: AffinePoint,
    change_message_commitment_value: AllocatedScalar, 
    change_message_commitment_blinder: AllocatedScalar,
    value: AllocatedScalar,
) {
    commitment(composer, commitment_two_value, commitment_two_blinder, commitment_two);
    commitment(composer, change_message_commitment_value, change_message_commitment_blinder, change_message_commitment);

    range(composer, commitment_two_value, 64);
    range(composer, change_message_commitment_value, 64);

    composer.add_gate(
        value.var,
        change_message_commitment_value.var,
        commitment_two_value.var,
        BlsScalar::one(),
        BlsScalar::one(),
        -BlsScalar::one(),
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
    fn obfuscated_to_contract_one() -> Result<(), Error> {

        let commitment_one_value = JubJubScalar::from(300 as u64);
        let commitment_one_blinder = JubJubScalar::from(100 as u64);
        let commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_one_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_one_blinder),
        );

        let message_spend_commitment_value = JubJubScalar::from(200 as u64);
        let message_spend_commitment_blinder = JubJubScalar::from(200 as u64);
        let message_spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * message_spend_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * message_spend_commitment_blinder),
        );

        let message_change_commitment_value = JubJubScalar::from(100 as u64);
        let message_change_commitment_blinder = JubJubScalar::from(300 as u64);
        let message_change_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * message_change_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * message_change_commitment_blinder),
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");


        let commitment_one_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
        let commitment_one_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let message_spend_commitment_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let message_spend_commitment_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let message_change_commitment_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let message_change_commitment_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));

        withdraw_from_obfuscated_to_contract_one(
            prover.mut_cs(),
            commitment_one,
            commitment_one_value, 
            commitment_one_blinder, 
            message_spend_commitment,
            message_spend_commitment_value, 
            message_spend_commitment_blinder,
            message_change_commitment, 
            message_change_commitment_value, 
            message_change_commitment_blinder,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        
        let mut verifier = Verifier::new(b"test");

        let commitment_one_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        let commitment_one_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let message_spend_commitment_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let message_spend_commitment_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let message_change_commitment_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let message_change_commitment_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        
        withdraw_from_obfuscated_to_contract_one(
            verifier.mut_cs(),
            commitment_one,
            commitment_one_value, 
            commitment_one_blinder, 
            message_spend_commitment,
            message_spend_commitment_value, 
            message_spend_commitment_blinder,
            message_change_commitment, 
            message_change_commitment_value, 
            message_change_commitment_blinder,
        );
        
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }

    #[test]
    fn obfuscated_to_contract_two() -> Result<(), Error> {

        let commitment_two_value = JubJubScalar::from(300 as u64);
        let commitment_two_blinder = JubJubScalar::from(100 as u64);
        let commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_two_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_two_blinder),
        );

        let change_message_commitment_value = JubJubScalar::from(200 as u64);
        let change_message_commitment_blinder = JubJubScalar::from(200 as u64);
        let change_message_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * change_message_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * change_message_commitment_blinder),
        );


        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");

        let commitment_two_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
        let commitment_two_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let change_message_commitment_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let change_message_commitment_blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let value = 
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        
            withdraw_from_obfuscated_to_contract_two(
            prover.mut_cs(),
            commitment_two,
            commitment_two_value,
            commitment_two_blinder,
            change_message_commitment,
            change_message_commitment_value,
            change_message_commitment_blinder,
            value,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        
        let mut verifier = Verifier::new(b"test");

        let commitment_two_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        let commitment_two_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let change_message_commitment_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let change_message_commitment_blinder =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let value = 
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        
            withdraw_from_obfuscated_to_contract_two(
            verifier.mut_cs(),
            commitment_two,
            commitment_two_value,
            commitment_two_blinder,
            change_message_commitment,
            change_message_commitment_value,
            change_message_commitment_blinder,
            value,
        );

        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }

}