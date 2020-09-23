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


/// This gadget constructs the circuit for a "Withdraw from Obfuscated" transaction.
pub fn withdraw_from_contract_obfuscated(
    composer: &mut StandardComposer,
    spend_commitment_value: AllocatedScalar, 
    spend_commitment_blinder: AllocatedScalar,
    spend_commitment: AffinePoint,
    message_commitment_value: AllocatedScalar,
    message_commitment_blinder: AllocatedScalar,
    message_commitment: AffinePoint,
    note_commitment_value: AllocatedScalar,
    note_commitment_blinder: AllocatedScalar,
    note_commitment: AffinePoint,
) {
    
    commitment(composer, spend_commitment_value, spend_commitment_blinder, spend_commitment);
    commitment(composer, message_commitment_value, message_commitment_blinder, message_commitment);
    commitment(composer, note_commitment_value, note_commitment_blinder, note_commitment);

    range(composer, spend_commitment_value, 64);
    range(composer, message_commitment_value, 64);
    range(composer, note_commitment_value, 64);

    composer.add_gate(
        message_commitment_value.var,
        note_commitment_value.var,
        spend_commitment_value.var,
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
    fn test_withdraw_from_obfuscated() -> Result<(), Error> {
        
        let spend_value = JubJubScalar::from(300 as u64);
        let spend_blinder = JubJubScalar::from(100 as u64);
        let spend_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * spend_value)
                + &(GENERATOR_NUMS_EXTENDED * spend_blinder),
        );

        let message_value = JubJubScalar::from(200 as u64);
        let message_blinder = JubJubScalar::from(200 as u64);
        let message_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * message_value)
                + &(GENERATOR_NUMS_EXTENDED * message_blinder),
        );

        let note_value = JubJubScalar::from(100 as u64);
        let note_blinder = JubJubScalar::from(300 as u64);
        let note_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * note_value)
                + &(GENERATOR_NUMS_EXTENDED * note_blinder),
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");


        let spend_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));
        let blind_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let message_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let message_blind =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(200));
        let note_value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let note_blind =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(300));

        withdraw_from_contract_obfuscated(
            prover.mut_cs(),
            spend_value, 
            blind_value, 
            spend_commitment,
            message_value, 
            message_blind,
            message_commitment, 
            note_value, 
            note_blind,
            note_commitment,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        
        let mut verifier = Verifier::new(b"test");

        let spend_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        let blind_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let message_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let message_blind =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(200));
        let note_value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let note_blind =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(300));
        
        withdraw_from_contract_obfuscated(
            verifier.mut_cs(),
            spend_value, 
            blind_value, 
            spend_commitment,
            message_value, 
            message_blind,
            message_commitment, 
            note_value, 
            note_blind,
            note_commitment,
        );  
        
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}

