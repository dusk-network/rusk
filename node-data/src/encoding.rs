// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, Read, Write};

use crate::bls::PublicKeyBytes;
use crate::ledger::{
    Attestation, Block, Fault, Header, IterationsInfo, Label,
    LedgerTransaction, Signature, SpentTransaction, StepVotes,
};
use crate::message::payload::{
    QuorumType, Ratification, RatificationResult, ValidationQuorum,
    ValidationResult, Vote,
};
use crate::message::{
    ConsensusHeader, MESSAGE_MAX_FAILED_ITERATIONS, SignInfo,
};
use crate::{
    MAX_NUMBER_OF_FAULTS, MAX_NUMBER_OF_TRANSACTIONS, MAX_SPENT_TX_ERROR_BYTES,
    Serializable,
};

const MAX_TX_LENGTH_BYTES: usize = 2 * 1024 * 1024;

/// Decoded outer transaction envelope used while reconstructing a
/// [`LedgerTransaction`].
struct RawTransaction {
    version: u32,
    tx_type: u32,
    protocol_tx: Vec<u8>,
}

fn read_raw_transaction<R: Read>(r: &mut R) -> io::Result<RawTransaction> {
    Ok(RawTransaction {
        version: LedgerTransaction::read_u32_le(r)?,
        tx_type: LedgerTransaction::read_u32_le(r)?,
        protocol_tx: LedgerTransaction::read_var_le_bytes32(
            r,
            MAX_TX_LENGTH_BYTES,
        )?,
    })
}

fn decode_transaction(
    raw: RawTransaction,
    decode_inner: impl FnOnce(&[u8]) -> Result<LedgerTransaction, dusk_bytes::Error>,
) -> io::Result<LedgerTransaction> {
    let mut tx = decode_inner(&raw.protocol_tx[..])
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    tx.version = raw.version;
    tx.r#type = raw.tx_type;
    Ok(tx)
}

fn read_protocol_transaction<R: Read>(
    r: &mut R,
    decode_inner: impl FnOnce(&[u8]) -> Result<LedgerTransaction, dusk_bytes::Error>,
) -> io::Result<LedgerTransaction> {
    decode_transaction(read_raw_transaction(r)?, decode_inner)
}

fn read_spent_transaction_fields<R: Read>(
    r: &mut R,
) -> io::Result<(RawTransaction, u64, u64, Option<String>)> {
    let raw = read_raw_transaction(r)?;
    let block_height = SpentTransaction::read_u64_le(r)?;
    let gas_spent = SpentTransaction::read_u64_le(r)?;
    let error_len = SpentTransaction::read_u32_le(r)?;
    if error_len as usize > MAX_SPENT_TX_ERROR_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SpentTransaction error string too large: {error_len} > {MAX_SPENT_TX_ERROR_BYTES}"
            ),
        ));
    }

    let err = if error_len > 0 {
        let mut buf = vec![0u8; error_len as usize];
        r.read_exact(&mut buf[..])?;

        Some(String::from_utf8(buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid utf-8 in SpentTransaction error string",
            )
        })?)
    } else {
        None
    };

    Ok((raw, block_height, gas_spent, err))
}

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
        if tx_len as usize > MAX_NUMBER_OF_TRANSACTIONS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "too many block transactions: {tx_len} > {MAX_NUMBER_OF_TRANSACTIONS}"
                ),
            ));
        }

        let txs = (0..tx_len)
            .map(|_| LedgerTransaction::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        // Read faults count
        let faults_len = Self::read_u32_le(r)?;
        if faults_len as usize > MAX_NUMBER_OF_FAULTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "too many block faults: {faults_len} > {MAX_NUMBER_OF_FAULTS}"
                ),
            ));
        }

        let faults = (0..faults_len)
            .map(|_| Fault::read(r))
            .collect::<Result<Vec<_>, _>>()?;

        Block::new(header, txs, faults)
    }
}

impl Serializable for LedgerTransaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        // Write version
        w.write_all(&self.version.to_le_bytes())?;

        // Write TxType
        w.write_all(&self.r#type.to_le_bytes())?;

        let data = self.protocol_bytes();

        // Write inner transaction
        Self::write_var_le_bytes32(w, &data)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        read_protocol_transaction(r, LedgerTransaction::decode_any)
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
                w.write_all(&0_u32.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let (raw, block_height, gas_spent, err) =
            read_spent_transaction_fields(r)?;
        let inner = decode_transaction(raw, |bytes| {
            LedgerTransaction::decode_for_ledger(bytes, block_height)
        })?;

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
                    ));
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
        self.step_votes.write(w)?;
        self.vote.write(w)?;
        self.quorum.write(w)?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let step_votes: StepVotes = StepVotes::read(r)?;
        let vote = Vote::read(r)?;
        let quorum = QuorumType::read(r)?;

        Ok(ValidationResult::new(step_votes, vote, quorum))
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
    use std::io;

    use dusk_core::transfer::TransactionFormat;
    use fake::{Dummy, Fake, Faker};

    use super::*;
    use crate::hard_fork::{
        set_aegis_activation_height, set_boreas_activation_height,
    };
    use crate::message::payload::{Candidate, Validation};

    const HISTORICAL_PRE_AEGIS_TX_HEX: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test-fixtures/pre_aegis_3422299.tx.hex"
    ));

    /// Asserts if encoding/decoding of a serializable type runs properly.
    fn assert_serializable<S: Dummy<Faker> + Eq + Serializable>() {
        let obj: S = Faker.fake();
        let mut buf = vec![];
        obj.write(&mut buf).expect("should be writable");

        assert!(obj.eq(&S::read(&mut &buf[..]).expect("should be readable")));
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
        assert_serializable::<LedgerTransaction>();
    }

    fn assert_transaction_roundtrip(
        tx: LedgerTransaction,
    ) -> LedgerTransaction {
        let mut buf = vec![];
        tx.write(&mut buf).expect("should be writable");

        let decoded =
            LedgerTransaction::read(&mut &buf[..]).expect("should decode");
        assert_eq!(decoded, tx);
        decoded
    }

    #[test]
    fn test_encoding_transaction_boreas() {
        let tx: LedgerTransaction =
            LedgerTransaction::from_protocol_with_format(
                Faker.fake::<LedgerTransaction>().protocol().clone(),
                TransactionFormat::Boreas,
            );
        let decoded = assert_transaction_roundtrip(tx);
        assert_eq!(decoded.format(), TransactionFormat::Boreas);
    }

    #[test]
    fn test_encoding_block_with_pre_aegis_transaction() {
        let header: Header = Faker.fake();
        let tx = LedgerTransaction::decode_any(
            &hex::decode(HISTORICAL_PRE_AEGIS_TX_HEX.trim())
                .expect("fixture hex should decode"),
        )
        .expect("historical tx should decode");
        let block = Block::new(header, vec![tx], vec![])
            .expect("block construction should succeed");

        let mut buf = vec![];
        block
            .write(&mut buf)
            .expect("block encoding should succeed");

        let decoded = Block::read(&mut &buf[..])
            .expect("block decoding should preserve legacy transactions");
        assert_eq!(decoded.txs().len(), 1);
        assert_eq!(decoded.txs()[0].format(), TransactionFormat::PreAegis);
        assert_eq!(decoded.txs()[0].id(), block.txs()[0].id());
    }

    #[test]
    fn test_encoding_spent_transaction() {
        assert_serializable::<SpentTransaction>();
    }

    #[test]
    fn test_encoding_spent_transaction_boreas() {
        set_aegis_activation_height(10);
        set_boreas_activation_height(200);

        let tx: LedgerTransaction =
            LedgerTransaction::from_protocol_with_format(
                Faker.fake::<LedgerTransaction>().protocol().clone(),
                TransactionFormat::Boreas,
            );
        let spent_tx = SpentTransaction {
            inner: tx,
            block_height: 200,
            gas_spent: 3,
            err: None,
        };

        let mut buf = vec![];
        spent_tx.write(&mut buf).expect("should be writable");
        let decoded =
            SpentTransaction::read(&mut &buf[..]).expect("should decode");
        assert_eq!(decoded, spent_tx);
        assert_eq!(decoded.inner.format(), TransactionFormat::Boreas);
    }

    #[test]
    fn test_spent_transaction_rejects_malformed_error_string() {
        fn decode_err(
            tx: &LedgerTransaction,
            error_len: u32,
            error_bytes: &[u8],
        ) -> io::Result<SpentTransaction> {
            let mut bytes = vec![];
            tx.write(&mut bytes)
                .expect("transaction encoding should succeed");
            bytes.extend_from_slice(&123_u64.to_le_bytes()); // block_height
            bytes.extend_from_slice(&456_u64.to_le_bytes()); // gas_spent
            bytes.extend_from_slice(&error_len.to_le_bytes());
            bytes.extend_from_slice(error_bytes);
            SpentTransaction::read(&mut &bytes[..])
        }

        let tx: LedgerTransaction = Faker.fake();

        let err = decode_err(&tx, 2, &[0xFF, 0xFF])
            .expect_err("invalid utf-8 error bytes must be rejected");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        let err = decode_err(&tx, (MAX_SPENT_TX_ERROR_BYTES as u32) + 1, &[])
            .expect_err("oversized error string must be rejected");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_spent_transaction_none_error_encoding_has_no_trailing_bytes() {
        let mut spent_tx: SpentTransaction = Faker.fake();
        spent_tx.err = None;

        let mut bytes = vec![];
        spent_tx
            .write(&mut bytes)
            .expect("spent transaction encoding should succeed");

        let mut slice = &bytes[..];
        let decoded = SpentTransaction::read(&mut slice)
            .expect("spent transaction decoding should succeed");
        assert!(decoded.err.is_none());
        assert!(slice.is_empty(), "deserializer left trailing bytes");
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

    #[test]
    fn test_rejects_oversized_lengths() {
        fn assert_invalid_data<T>(result: io::Result<T>, reason: &str) {
            match result {
                Ok(_) => panic!("{reason}"),
                Err(err) => {
                    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
                }
            }
        }

        let mut tx_bytes = vec![];
        tx_bytes.extend_from_slice(&1_u32.to_le_bytes()); // version
        tx_bytes.extend_from_slice(&1_u32.to_le_bytes()); // tx type
        tx_bytes.extend_from_slice(
            &((MAX_TX_LENGTH_BYTES as u32) + 1).to_le_bytes(),
        );
        assert_invalid_data(
            LedgerTransaction::read(&mut &tx_bytes[..]),
            "oversized length-prefixed tx payload must be rejected",
        );

        let mut block_txs_bytes = vec![];
        Header::default()
            .write(&mut block_txs_bytes)
            .expect("header encoding should succeed");
        block_txs_bytes.extend_from_slice(
            &((MAX_NUMBER_OF_TRANSACTIONS as u32) + 1).to_le_bytes(),
        );
        assert_invalid_data(
            Block::read(&mut &block_txs_bytes[..]),
            "block with too many transactions must be rejected",
        );

        let mut block_faults_bytes = vec![];
        Header::default()
            .write(&mut block_faults_bytes)
            .expect("header encoding should succeed");
        block_faults_bytes.extend_from_slice(&0_u32.to_le_bytes()); // tx_len
        block_faults_bytes.extend_from_slice(
            &((MAX_NUMBER_OF_FAULTS as u32) + 1).to_le_bytes(),
        );
        assert_invalid_data(
            Block::read(&mut &block_faults_bytes[..]),
            "block with too many faults must be rejected",
        );
    }
}
