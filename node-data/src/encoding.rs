// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ledger::*;
use crate::Serializable;
use std::io::{self, Read, Write};

impl Serializable for Block {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header().write(w)?;

        let txs_num = self.txs.len() as u8;
        w.write_all(&txs_num.to_le_bytes())?;

        for t in self.txs.iter() {
            t.write(w)?;
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = Header::read(r)?;

        // Read transactions count
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        let txs = (0..buf[0] as usize)
            .map(|_| Transaction::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        Block::new(header, txs)
    }
}

impl Serializable for Transaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let data = self.inner.to_var_bytes();

        // Write inner transaction
        let len = data.len() as u32;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(&data)?;

        // Write gas_spent
        match self.gas_spent {
            Some(gas_spent) => {
                w.write_all(&1_u8.to_le_bytes())?;
                w.write_all(&gas_spent.to_le_bytes())?;
            }
            None => {
                w.write_all(&0_u8.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;

        let len = u32::from_le_bytes(buf);
        let mut buf = vec![0u8; len as usize];
        r.read_exact(&mut buf)?;

        let inner = dusk_wallet_core::Transaction::from_slice(&buf[..])
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut optional = [0u8; 1];
        r.read_exact(&mut optional)?;

        let gas_spent = if optional[0] != 0 {
            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;

            Some(u64::from_le_bytes(buf))
        } else {
            None
        };

        Ok(Self { inner, gas_spent })
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.marshal_hashable(w, false)?;
        self.cert.write(w)?;
        w.write_all(&self.hash[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut header = Self::unmarshal_hashable(r)?;
        header.cert = Certificate::read(r)?;
        r.read_exact(&mut header.hash[..])?;
        Ok(header)
    }
}

impl Serializable for Certificate {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        // In order to be aligned with golang impl,
        // we cannot use here StepVotes::write for now.
        Self::write_var_le_bytes(
            w,
            &self.first_reduction.signature.inner()[..],
        )?;
        Self::write_var_le_bytes(
            w,
            &self.second_reduction.signature.inner()[..],
        )?;

        w.write_all(&self.first_reduction.bitset.to_le_bytes())?;
        w.write_all(&self.second_reduction.bitset.to_le_bytes())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let first_red_signature: [u8; 48] = Self::read_var_le_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let second_red_signature: [u8; 48] = Self::read_var_le_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let first_red_bitset = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let sec_red_bitset = u64::from_le_bytes(buf);

        Ok(Certificate {
            first_reduction: StepVotes {
                bitset: first_red_bitset,
                signature: Signature(first_red_signature),
            },
            second_reduction: StepVotes {
                bitset: sec_red_bitset,
                signature: Signature(second_red_signature),
            },
        })
    }
}

impl Serializable for StepVotes {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.bitset.to_le_bytes())?;
        Self::write_var_le_bytes(w, &self.signature.inner()[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let signature: [u8; 48] = Self::read_var_le_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(StepVotes {
            bitset: u64::from_le_bytes(buf),
            signature: Signature(signature),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Dummy, Fake, Faker};

    /// Asserts if encoding/decoding of a serializable type runs properly.
    fn assert_serializable<S: Dummy<Faker> + Eq + Serializable>() {
        let obj: S = Faker.fake();
        let mut buf = vec![];
        obj.write(&mut buf).expect("should be writable");

        assert!(obj
            .eq(&S::read(&mut &buf.to_vec()[..]).expect("should be readable")));
    }

    #[test]
    fn test_encoding_cert() {
        assert_serializable::<Certificate>();
    }

    #[test]
    fn test_encoding_transaction() {
        assert_serializable::<Transaction>();
    }

    #[test]
    fn test_encoding_header() {
        assert_serializable::<Header>();
    }

    #[test]
    fn test_encoding_block() {
        assert_serializable::<Block>();
    }
}
