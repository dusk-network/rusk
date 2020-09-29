// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use dusk_pki::jubjub_decode;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::{
    AffinePoint as JubJubAffine, ExtendedPoint as JubJubExtended,
};
use poseidon252::cipher::PoseidonCipher;
use std::io::{self, Read, Write};

/// The crossover note, contained in a Phoenix transaction.
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl Read for Crossover {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        n +=
            buf.write(&JubJubAffine::from(self.value_commitment).to_bytes())?;
        n += buf.write(&self.nonce.to_bytes())?;
        n += buf.write(&self.encrypted_data.to_bytes())?;

        Ok(n)
    }
}

impl Write for Crossover {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_scalar = [0u8; 32];
        let mut one_cipher = [0u8; 96];

        n += buf.read(&mut one_scalar)?;
        self.value_commitment =
            JubJubExtended::from(jubjub_decode::<JubJubAffine>(&one_scalar)?);

        n += buf.read(&mut one_scalar)?;
        self.nonce = Option::from(BlsScalar::from_bytes(&one_scalar)).ok_or(
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize anchor",
            ),
        )?;

        n += buf.read(&mut one_cipher)?;
        self.encrypted_data =
            PoseidonCipher::from_bytes(&one_cipher).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not deserialize encrypted data",
            ))?;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
