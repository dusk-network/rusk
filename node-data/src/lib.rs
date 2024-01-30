// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod bls;
pub mod encoding;
pub mod ledger;
pub mod message;

use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepName {
    Proposal = 0,
    Validation = 1,
    Ratification = 2,
}

impl StepName {
    pub fn to_step(self, iteration: u8) -> u16 {
        iteration as u16 * 3 + (self as u16)
    }
}

pub trait Serializable {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()>;
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized;

    fn read_bytes<R: Read, const N: usize>(r: &mut R) -> io::Result<[u8; N]> {
        let mut buffer = [0u8; N];
        r.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
        let mut num = [0u8; 1];
        r.read_exact(&mut num)?;
        Ok(num[0])
    }

    fn read_u64_le<R: Read>(r: &mut R) -> io::Result<u64> {
        let data = Self::read_bytes(r)?;
        Ok(u64::from_le_bytes(data))
    }
    fn read_u32_le<R: Read>(r: &mut R) -> io::Result<u32> {
        let data = Self::read_bytes(r)?;
        Ok(u32::from_le_bytes(data))
    }

    /// Writes length-prefixed fields
    fn write_var_le_bytes32<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u32;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;
        Ok(())
    }

    /// Reads length-prefixed fields
    fn read_var_le_bytes32<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let len = Self::read_u32_le(r)? as usize;

        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
}
