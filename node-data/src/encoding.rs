// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, Read, Write};

use execution_core::transfer::Transaction as ProtocolTransaction;

use crate::bls::PublicKeyBytes;
use crate::ledger::{
    Attestation, Block, Fault, Header, IterationsInfo, Label, Signature,
    SpentTransaction, StepVotes, Transaction,
};
use crate::message::payload::{
    QuorumType, Ratification, RatificationResult, ValidationQuorum,
    ValidationResult, Vote,
};
use crate::message::{
    ConsensusHeader, SignInfo, MESSAGE_MAX_FAILED_ITERATIONS,
};
use crate::Serializable;

impl Serializable for Block {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header().write(w)?;

        let txs_len = self.txs().len() as u32;
        w.write_all(&txs_len.to_le_bytes())?;

        for t in self.txs().iter() {
            t.write(w)?;
        }

        let faults_len = self.faults().len() as u32;
        w.write_all(&faults_len.to_le_bytes())?;

        for f in self.faults().iter() {
            f.write(w)?;
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

        // Read faults count
        let faults_len = Self::read_u32_le(r)?;

        let faults = (0..faults_len)
            .map(|_| Fault::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        Block::new(header, txs, faults)
    }
}

impl Serializable for Transaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        // Write version
        w.write_all(&self.version.to_le_bytes())?;

        // Write TxType
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

        let protocol_tx = Self::read_var_le_bytes32(r)?;
        let tx_size = protocol_tx.len();
        let inner = ProtocolTransaction::from_slice(&protocol_tx[..])
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(Self {
            inner,
            version,
            r#type: tx_type,
            size: Some(tx_size),
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
        self.att.write(w)?;
        w.write_all(&self.hash)?;
        w.write_all(self.signature.inner())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut header = Self::unmarshal_hashable(r)?;
        header.att = Attestation::read(r)?;
        header.hash = Self::read_bytes(r)?;
        header.signature = Signature::from(Self::read_bytes(r)?);
        Ok(header)
    }
}

impl Serializable for Attestation {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.result.write(w)?;
        self.validation.write(w)?;
        self.ratification.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let result = RatificationResult::read(r)?;
        let validation = StepVotes::read(r)?;
        let ratification = StepVotes::read(r)?;

        Ok(Attestation {
            result,
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

impl Serializable for RatificationResult {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            RatificationResult::Fail(v) => {
                w.write_all(&[0])?;
                v.write(w)?;
            }

            RatificationResult::Success(v) => {
                w.write_all(&[1])?;
                v.write(w)?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let result = match Self::read_u8(r)? {
            0 => {
                let vote = Vote::read(r)?;
                Self::Fail(vote)
            }
            1 => {
                let vote = Vote::read(r)?;
                Self::Success(vote)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid RatificationResult",
            ))?,
        };
        Ok(result)
    }
}

impl Serializable for IterationsInfo {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let count = self.att_list.len() as u8;
        w.write_all(&count.to_le_bytes())?;

        for iter in &self.att_list {
            match iter {
                Some((att, pk)) => {
                    w.write_all(&[1])?;
                    att.write(w)?;
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
        let mut att_list = vec![];

        let count = Self::read_u8(r)?;

        // Iteration is 0-based
        if count > MESSAGE_MAX_FAILED_ITERATIONS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid iterations_info count {count})"),
            ));
        }

        for _ in 0..count {
            let opt = Self::read_u8(r)?;

            let att = match opt {
                0 => None,
                1 => {
                    let att = Attestation::read(r)?;
                    let pk = Self::read_bytes(r)?;
                    Some((att, PublicKeyBytes(pk)))
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid option",
                    ))
                }
            };
            att_list.push(att)
        }

        Ok(IterationsInfo { att_list })
    }
}

impl Serializable for Label {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Label::Accepted(v) => {
                w.write_all(&0u8.to_le_bytes())?;
                w.write_all(&v.to_le_bytes())?;
            }
            Label::Attested(v) => {
                w.write_all(&1u8.to_le_bytes())?;
                w.write_all(&v.to_le_bytes())?;
            }
            Label::Confirmed(v) => {
                w.write_all(&2u8.to_le_bytes())?;
                w.write_all(&v.to_le_bytes())?;
            }
            Label::Final(v) => {
                w.write_all(&3u8.to_le_bytes())?;
                w.write_all(&v.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let label = Self::read_u8(r)?;
        let label = match label {
            0 => Label::Accepted(Self::read_u64_le(r)?),
            1 => Label::Attested(Self::read_u64_le(r)?),
            2 => Label::Confirmed(Self::read_u64_le(r)?),
            3 => Label::Final(Self::read_u64_le(r)?),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid label",
            ))?,
        };

        Ok(label)
    }
}

impl Serializable for Ratification {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;
        self.vote.write(w)?;
        w.write_all(&self.timestamp.to_le_bytes())?;
        self.validation_result.write(w)?;
        // sign_info at the end
        self.sign_info.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = ConsensusHeader::read(r)?;
        let vote = Vote::read(r)?;
        let timestamp = Self::read_u64_le(r)?;
        let validation_result = ValidationResult::read(r)?;
        let sign_info = SignInfo::read(r)?;

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

        Ok(ValidationResult::new(sv, vote, quorum))
    }
}

impl Serializable for ValidationQuorum {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;
        self.result.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = ConsensusHeader::read(r)?;
        let result = ValidationResult::read(r)?;

        Ok(ValidationQuorum { header, result })
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
    fn test_encoding_att() {
        assert_serializable::<Attestation>();
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

    #[test]
    fn test_encoding_ratification_result() {
        assert_serializable::<RatificationResult>();
    }

    #[test]
    fn test_encoding_fault() {
        assert_serializable::<Fault>();
    }
}
