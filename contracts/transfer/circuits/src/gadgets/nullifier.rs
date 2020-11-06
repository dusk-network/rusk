// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;

/// Prove knowledge of the preimage of a given nullifier.
pub fn nullifier_gadget(
    composer: &mut StandardComposer,
    pos: AllocatedScalar,
    sk: AllocatedScalar,
) -> Variable {
    sponge_hash_gadget(composer, &[sk.var, pos.var])
}

#[cfg(test)]
mod nullifier_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use phoenix_core::{Note, NoteType};
    use poseidon252::sponge::sponge::sponge_hash;
    #[test]
    fn test_nullifier() -> Result<(), Error> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk = SecretSpendKey::new(secret1, secret2);
        let psk = PublicSpendKey::from(ssk);
        let value = 25u64;
        let note = Note::new(NoteType::Transparent, &psk, value);
        let sk_r = BlsScalar::from(ssk.sk_r(note.stealth_address()));
        let pos = note.pos();
        let nullifier_hash = note.gen_nullifier(&ssk);

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;

        let mut prover = Prover::new(b"test");

        let position_1 =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(pos));
        let sk = AllocatedScalar::allocate(prover.mut_cs(), sk_r);
        let nul_scalar = sponge_hash(&[sk.scalar, position_1.scalar]);
        let nul = AllocatedScalar::allocate(prover.mut_cs(), nul_scalar);

        nullifier_gadget(prover.mut_cs(), position_1, sk);
        prover.mut_cs().constrain_to_constant(
            nul.var,
            BlsScalar::zero(),
            -nullifier_hash,
        );

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let pos =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(pos));
        let sk1 = AllocatedScalar::allocate(verifier.mut_cs(), sk_r);
        let nul_scalar = sponge_hash(&[sk.scalar, pos.scalar]);
        let nul = AllocatedScalar::allocate(verifier.mut_cs(), nul_scalar);

        nullifier_gadget(verifier.mut_cs(), pos, sk1);

        verifier.mut_cs().constrain_to_constant(
            nul.var,
            BlsScalar::zero(),
            -nullifier_hash,
        );
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
