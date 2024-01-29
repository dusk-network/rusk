// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::bls::PublicKeyBytes;
use crate::ledger::{
    Block, Certificate, Header, IterationsInfo, Label, SpentTransaction,
    StepVotes, Transaction,
};
use crate::message::payload::{
    QuorumType, Ratification, ValidationResult, Vote,
};
use crate::message::{ConsensusHeader, SignInfo};
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
        let tx_len = Self::read_u32_le(r)?;

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
        let version = Self::read_u32_le(r)?;
        let tx_type = Self::read_u32_le(r)?;

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
                w.write_all(&(b.len() as u32).to_le_bytes())?;
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

        let block_height = Self::read_u64_le(r)?;
        let gas_spent = Self::read_u64_le(r)?;
        let error_len = Self::read_u32_le(r)?;

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
        w.write_all(&self.hash)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut header = Self::unmarshal_hashable(r)?;
        header.cert = Certificate::read(r)?;
        header.hash = Self::read_bytes(r)?;
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
        w.write_all(self.aggregate_signature.inner())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let bitset = Self::read_u64_le(r)?;
        let aggregate_signature = Self::read_bytes(r)?;

        Ok(StepVotes {
            bitset,
            aggregate_signature: aggregate_signature.into(),
        })
    }
}

impl Serializable for IterationsInfo {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let count = self.cert_list.len() as u8;
        w.write_all(&count.to_le_bytes())?;

        for iter in &self.cert_list {
            match iter {
                Some((cert, pk)) => {
                    w.write_all(&[1])?;
                    cert.write(w)?;
                    w.write_all(pk.inner())?;
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

        let count = Self::read_u8(r)?;

        for _ in 0..count {
            let opt = Self::read_u8(r)?;

            let cert = match opt {
                0 => None,
                1 => {
                    let cert = Certificate::read(r)?;
                    let pk = Self::read_bytes(r)?;
                    Some((cert, PublicKeyBytes(pk)))
                }
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
        let label = Self::read_u8(r)?;
        Ok(label.into())
    }
}

impl Serializable for Ratification {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;
        self.vote.write(w)?;
        self.sign_info.write(w)?;
        w.write_all(&self.timestamp.to_le_bytes())?;
        self.validation_result.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = ConsensusHeader::read(r)?;
        let vote = Vote::read(r)?;
        let sign_info = SignInfo::read(r)?;
        let timestamp = Self::read_u64_le(r)?;
        let validation_result = ValidationResult::read(r)?;

        Ok(Ratification {
            header,
            vote,
            sign_info,
            timestamp,
            validation_result,
        })
    }
}

impl Serializable for ValidationResult {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.sv.write(w)?;
        self.vote.write(w)?;
        self.quorum.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let sv = StepVotes::read(r)?;
        let vote = Vote::read(r)?;
        let quorum = QuorumType::read(r)?;

        Ok(ValidationResult { sv, vote, quorum })
    }
}

impl Serializable for QuorumType {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let val: u8 = *self as u8;
        w.write_all(&val.to_le_bytes())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::read_u8(r)?.into())
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
        assert_serializable::<ConsensusHeader>();
    }

    #[test]
    fn test_encoding_block() {
        assert_serializable::<Block>();
    }
}
