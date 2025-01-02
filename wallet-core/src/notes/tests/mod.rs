// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(test)]
pub(crate) mod helpers {
    use dusk_core::transfer::phoenix::{Note, PublicKey as PhoenixPublicKey};
    use dusk_core::JubJubScalar;
    use ff::Field;
    use rand::{CryptoRng, RngCore};

    /// Helper function to generate test notes with specific parameters.
    /// Used across different test modules to ensure consistent note generation.
    pub(crate) fn gen_note<R>(
        rng: &mut R,
        owner_pk: &PhoenixPublicKey,
        value: u64,
        is_obfuscated: bool,
    ) -> Note
    where
        R: RngCore + CryptoRng,
    {
        let value_blinder = JubJubScalar::random(&mut *rng);
        let blinder1 = JubJubScalar::random(&mut *rng);
        let blinder2 = JubJubScalar::random(&mut *rng);
        let sender_blinder = [blinder1, blinder2];

        if is_obfuscated {
            Note::obfuscated(
                &mut *rng,
                owner_pk,
                owner_pk,
                value,
                value_blinder,
                sender_blinder,
            )
        } else {
            Note::transparent(
                &mut *rng,
                owner_pk,
                owner_pk,
                value,
                sender_blinder,
            )
        }
    }
}

pub(crate) mod property_tests;
