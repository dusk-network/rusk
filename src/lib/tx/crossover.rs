// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::ExtendedPoint as JubJubExtended;
use poseidon252::cipher::PoseidonCipher;

/// The crossover note, contained in a Phoenix transaction.
#[derive(Debug, Clone, Copy)]
pub struct Crossover {
    value_commitment: JubJubExtended,
    nonce: BlsScalar,
    encrypted_data: PoseidonCipher,
}

impl Default for Crossover {
    fn default() -> Self {
        Crossover {
            value_commitment: JubJubExtended::identity(),
            nonce: BlsScalar::zero(),
            encrypted_data: PoseidonCipher::default(),
        }
    }
}

impl Crossover {
    /// Create a new Crossover note with the given parameters.
    pub fn new(
        value_commitment: JubJubExtended,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
    ) -> Self {
        Crossover {
            value_commitment,
            nonce,
            encrypted_data,
        }
    }

    /// Get the crossover note's value commitment.
    pub fn value_commitment(&self) -> JubJubExtended {
        self.value_commitment
    }

    /// Get the crossover note's nonce.
    pub fn nonce(&self) -> BlsScalar {
        self.nonce
    }

    /// Get the crossover note's encrypted data.
    pub fn encrypted_data(&self) -> PoseidonCipher {
        self.encrypted_data
    }
}
