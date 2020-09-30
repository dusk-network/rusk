// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

/// This gadget simply wraps around the composer's `range_gate` function,
/// but takes in any type that implements the traits of the note,
/// for ease-of-use in circuit construction.
pub fn range(
    composer: &mut StandardComposer,
    value: AllocatedScalar,
    bit_length: usize,
) {
    composer.range_gate(value.var, bit_length);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;

    #[test]
    fn range_gadget() -> Result<(), Error> {
        let value: u64 = rand::thread_rng().gen();

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");

        let val =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(value));

        range(prover.mut_cs(), val, 64);

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let val = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(value),
        );

        range(verifier.mut_cs(), val, 64);
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
