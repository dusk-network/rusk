// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use dusk_pki::{PublicSpendKey, StealthAddress};
use dusk_plonk::jubjub::Scalar as JubJubScalar;
use std::io::{self, Read, Write};

/// The fee note, contained in a Phoenix transaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fee {
    gas_limit: u64,
    gas_price: u64,
    address: StealthAddress,
}

impl Default for Fee {
    fn default() -> Self {
        let r = JubJubScalar::random(&mut rand::thread_rng());
        Fee {
            gas_limit: 0,
            gas_price: 0,
            address: PublicSpendKey::default().gen_stealth_address(&r),
        }
    }
}

impl Fee {
    /// Create a new Fee, with the given parameters.
    pub fn new(
        gas_limit: u64,
        gas_price: u64,
        address: StealthAddress,
    ) -> Self {
        Fee {
            gas_limit,
            gas_price,
            address,
        }
    }

    /// Get the fee's gas limit.
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// Get the fee's gas price.
    pub fn gas_price(&self) -> u64 {
        self.gas_price
    }

    /// Get the fee's return address.
    pub fn address(&self) -> StealthAddress {
        self.address
    }
}

impl Read for Fee {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        n += buf.write(&self.gas_limit.to_le_bytes())?;
        n += buf.write(&self.gas_price.to_le_bytes())?;
        n += buf.write(&self.address.to_bytes())?;

        Ok(n)
    }
}

impl Write for Fee {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let mut n = 0;

        let mut one_u64 = [0u8; 8];
        let mut one_stealth_address = [0u8; 64];

        n += buf.read(&mut one_u64)?;
        self.gas_limit = u64::from_le_bytes(one_u64);

        n += buf.read(&mut one_u64)?;
        self.gas_price = u64::from_le_bytes(one_u64);

        n += buf.read(&mut one_stealth_address)?;
        self.address = StealthAddress::from_bytes(&one_stealth_address)?;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
