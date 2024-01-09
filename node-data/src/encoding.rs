// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ledger::*;
use crate::message::payload::{QuorumType, Ratification, ValidationResult};
use crate::Serializable;
use std::io::{self, Read, Write};

impl Serializable for Block {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header().write(w)?;

        let txs_len = self.txs().len() as u32;
        w.write_all(&txs_len.to_le_bytes())?;

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
        let mut tx_len = [0u8; 4];
        r.read_exact(&mut tx_len)?;
        let tx_len = u32::from_le_bytes(tx_len);

        let txs = (0..tx_len)
            .map(|_| Transaction::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        Block::new(header, txs)
    }
}

impl Serializable for Transaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        //Write version
        w.write_all(&self.version.to_le_bytes())?;

        //Write TxType
        w.write_all(&self.r#type.to_le_bytes())?;

        let data = self.inner.to_var_bytes();

        // Write inner transaction
        Self::write_var_le_bytes32(w, &data)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut version = [0u8; 4];
        r.read_exact(&mut version)?;
        let version = u32::from_le_bytes(version);

        let mut tx_type = [0u8; 4];
        r.read_exact(&mut tx_type)?;
        let tx_type = u32::from_le_bytes(tx_type);

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
        self.marshal_hashable(w)?;
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
        self.validation.write(w)?;
        self.ratification.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let validation = StepVotes::read(r)?;
        let ratification = StepVotes::read(r)?;

        Ok(Certificate {
            validation,
            ratification,
        })
    }
}

impl Serializable for StepVotes {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.bitset.to_le_bytes())?;
        w.write_all(&self.aggregate_signature.inner())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut bitset = [0u8; 8];
        r.read_exact(&mut bitset[..])?;
        let bitset = u64::from_le_bytes(bitset);
        let mut aggregate_signature = [0u8; 48];
        r.read_exact(&mut aggregate_signature)?;

        Ok(StepVotes {
            bitset,
            aggregate_signature: Signature(aggregate_signature),
        })
    }
}

impl Serializable for IterationsInfo {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let count = self.cert_list.len() as u8;
        w.write_all(&count.to_le_bytes())?;

        for cert in &self.cert_list {
            match cert {
                Some(cert) => {
                    w.write_all(&[1])?;
                    cert.write(w)?;
                }
                None => w.write_all(&[0])?,
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut cert_list = vec![];

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let count = buf[0];

        for _ in 0..count {
            let mut opt = [0u8; 1];
            r.read_exact(&mut opt[..])?;

            let cert = match opt[0] {
                0 => None,
                1 => Some(Certificate::read(r)?),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid option",
                    ))
                }
            };
            cert_list.push(cert)
        }

        Ok(IterationsInfo { cert_list })
    }
}

impl From<u8> for Label {
    fn from(value: u8) -> Self {
        match value {
            0 => Label::Accepted,
            1 => Label::Attested,
            2 => Label::Final,
            _ => panic!("Invalid u8 value for Label"),
        }
    }
}

impl Serializable for Label {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let byte: u8 = (*self) as u8;
        w.write_all(&byte.to_le_bytes())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;

        Ok(buf[0].into())
    }
}

impl Serializable for Ratification {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.signature)?;
        self.validation_result.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut signature = [0u8; 48];
        r.read_exact(&mut signature[..])?;

        let validation_result = ValidationResult::read(r)?;

        Ok(Ratification {
            signature,
            validation_result,
        })
    }
}

impl Serializable for ValidationResult {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.sv.write(w)?;
        w.write_all(&self.hash[..])?;
        self.quorum.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let sv = StepVotes::read(r)?;

        let mut hash = [0u8; 32];
        r.read_exact(&mut hash)?;

        let quorum = QuorumType::read(r)?;

        Ok(ValidationResult { sv, hash, quorum })
    }
}

impl Serializable for QuorumType {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            QuorumType::ValidQuorum => w.write_all(&0u8.to_le_bytes())?,
            QuorumType::InvalidQuorum => w.write_all(&1u8.to_le_bytes())?,
            QuorumType::NilQuorum => w.write_all(&2u8.to_le_bytes())?,
            _ => w.write_all(&255u8.to_le_bytes())?,
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        Ok(match buf[0] {
            0 => QuorumType::ValidQuorum,
            1 => QuorumType::InvalidQuorum,
            2 => QuorumType::NilQuorum,
            _ => QuorumType::NoQuorum,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::message::payload::{Candidate, Validation};

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
    fn test_encoding_ratification() {
        assert_serializable::<Ratification>();
    }

    #[test]
    fn test_encoding_validation() {
        assert_serializable::<Validation>();
    }

    #[test]
    fn test_encoding_candidate() {
        assert_serializable::<Candidate>();
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
