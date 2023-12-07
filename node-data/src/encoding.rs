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

        Self::write_varint(w, self.txs().len() as u64)?;

        for t in self.txs().iter() {
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
        let txlen = Self::read_varint(r)?;

        let txs = (0..txlen)
            .map(|_| Transaction::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        Block::new(header, txs)
    }
}

impl Serializable for Transaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        //Write version
        w.write_all(&1u32.to_le_bytes())?;

        //Write TxType
        w.write_all(&1u32.to_le_bytes())?;

        let data = self.inner.to_var_bytes();

        // Write inner transaction
        let len = data.len() as u32;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(&data)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let version = u32::from_le_bytes(buf);

        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let tx_type = u32::from_le_bytes(buf);

        let tx_payload = Self::read_var_le_bytes32(r)?;
        let inner = phoenix_core::Transaction::from_slice(&tx_payload[..])
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(Self {
            inner,
            version,
            r#type: tx_type,
        })
    }
}

impl Serializable for SpentTransaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.inner.write(w)?;
        w.write_all(&self.block_height.to_le_bytes())?;
        w.write_all(&self.gas_spent.to_le_bytes())?;

        match &self.err {
            Some(e) => {
                let b = e.as_bytes();
                w.write_all(&(b.len() as u64).to_le_bytes())?;
                w.write_all(b)?;
            }
            None => {
                w.write_all(&0_u64.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let inner = Transaction::read(r)?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let block_height = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let gas_spent = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;

        let error_len = u64::from_le_bytes(buf);

        let err = if error_len > 0 {
            let mut buf = vec![0u8; error_len as usize];
            r.read_exact(&mut buf[..])?;

            Some(String::from_utf8(buf).expect("Cannot from_utf8"))
        } else {
            None
        };

        Ok(Self {
            inner,
            block_height,
            gas_spent,
            err,
        })
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
        Self::write_var_bytes(
            w,
            &self.validation.aggregate_signature.inner()[..],
        )?;
        Self::write_var_bytes(
            w,
            &self.ratification.aggregate_signature.inner()[..],
        )?;

        w.write_all(&self.validation.bitset.to_le_bytes())?;
        w.write_all(&self.ratification.bitset.to_le_bytes())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let validation_signature: [u8; 48] = Self::read_var_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let ratification_signature: [u8; 48] = Self::read_var_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let validation_bitset = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let ratification_bitset = u64::from_le_bytes(buf);

        Ok(Certificate {
            validation: StepVotes {
                bitset: validation_bitset,
                aggregate_signature: Signature(validation_signature),
            },
            ratification: StepVotes {
                bitset: ratification_bitset,
                aggregate_signature: Signature(ratification_signature),
            },
        })
    }
}

impl Serializable for StepVotes {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.bitset.to_le_bytes())?;
        Self::write_var_bytes(w, &self.aggregate_signature.inner()[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let aggregate_signature: [u8; 48] = Self::read_var_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(StepVotes {
            bitset: u64::from_le_bytes(buf),
            aggregate_signature: Signature(aggregate_signature),
        })
    }
}

impl Serializable for IterationsInfo {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let count = self.cert_list.len() as u8;
        w.write_all(&count.to_le_bytes())?;

        let count_of_some =
            self.cert_list.iter().filter(|cert| cert.is_some()).count() as u8;

        w.write_all(&count_of_some.to_le_bytes())?;

        for i in 0..self.cert_list.len() as u8 {
            if let Some(cert) = &self.cert_list[i as usize] {
                w.write_all(&i.to_le_bytes())?;
                cert.write(w)?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let count = buf[0];

        let mut certificates = vec![None; count as usize];

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let count_of_some = buf[0];

        for _i in 0..count_of_some {
            let mut buf = [0u8; 1];
            r.read_exact(&mut buf[..])?;
            let iter_num = buf[0] as usize;

            certificates[iter_num] = Some(Certificate::read(r)?);
        }

        Ok(IterationsInfo {
            cert_list: certificates,
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
    fn test_encoding_iterations_info() {
        assert_serializable::<IterationsInfo>();
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
    fn test_encoding_spent_transaction() {
        assert_serializable::<SpentTransaction>();
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
