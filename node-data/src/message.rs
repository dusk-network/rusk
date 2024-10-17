// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable as DuskSerializable;
use execution_core::signatures::bls::{
    Error as BlsSigError, MultisigPublicKey as BlsMultisigPublicKey,
    MultisigSignature as BlsMultisigSignature, PublicKey as BlsPublicKey,
    SecretKey as BlsSecretKey,
};
use payload::{Nonce, ValidationQuorum};
use tracing::{error, warn};

use crate::bls::PublicKey;
use crate::ledger::{to_str, Hash, Signature};
use crate::StepName;
use crate::{bls, ledger, Serializable};
use core::fmt;
use std::cmp::Ordering;
use std::io::{self, Read, Write};
use std::net::SocketAddr;

use async_channel::TrySendError;

use self::payload::{Candidate, Ratification, Validation};

/// Topic field position in the message binary representation
pub const TOPIC_FIELD_POS: usize = 1 + 2 + 2;
pub const PROTOCOL_VERSION: Version = Version(1, 0, 0);

/// Max value for iteration.
pub const MESSAGE_MAX_ITER: u8 = 50;

/// Block version
pub const BLOCK_HEADER_VERSION: u8 = 1;

/// Max value for failed iterations.
pub const MESSAGE_MAX_FAILED_ITERATIONS: u8 = 8;

#[derive(Debug, Clone)]
/// Represent version (major, minor, patch)
pub struct Version(pub u8, pub u16, pub u16);

impl Default for Version {
    fn default() -> Self {
        PROTOCOL_VERSION
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Version(maj, min, patch) = self;
        write!(f, "{maj}.{min}.{patch}")
    }
}

impl crate::Serializable for Version {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let Version(maj, min, patch) = self;
        w.write_all(&[*maj])?;
        w.write_all(&min.to_le_bytes())?;
        w.write_all(&patch.to_le_bytes())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let maj = Self::read_u8(r)?;
        let min = Self::read_u16_le(r)?;
        let patch = Self::read_u16_le(r)?;
        Ok(Self(maj, min, patch))
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Past,
    Present,
    Future,
}

impl From<Ordering> for Status {
    fn from(value: Ordering) -> Self {
        match value {
            Ordering::Less => Self::Past,
            Ordering::Equal => Self::Present,
            Ordering::Greater => Self::Future,
        }
    }
}

/// Message definition
#[derive(Debug, Default, Clone)]
pub struct Message {
    pub version: Version,
    topic: Topics,
    pub header: ConsensusHeader,
    pub payload: Payload,

    pub metadata: Option<Metadata>,
}

pub trait WireMessage: Into<Payload> {
    const TOPIC: Topics;
    fn consensus_header(&self) -> ConsensusHeader {
        ConsensusHeader::default()
    }
    fn payload(self) -> Payload {
        self.into()
    }
}

impl Message {
    pub fn compare(&self, round: u64, iteration: u8, step: StepName) -> Status {
        self.header
            .round
            .cmp(&round)
            .then_with(|| self.get_step().cmp(&step.to_step(iteration)))
            .into()
    }
    pub fn get_signer(&self) -> Option<bls::PublicKey> {
        let signer = match &self.payload {
            Payload::Candidate(c) => c.sign_info().signer,
            Payload::Validation(v) => v.sign_info().signer,
            Payload::Ratification(r) => r.sign_info().signer,
            msg => {
                warn!("Calling get_signer for {msg:?}");
                return None;
            }
        };
        Some(signer)
    }
    pub fn get_step(&self) -> u8 {
        match &self.payload {
            Payload::Candidate(c) => c.get_step(),
            Payload::Validation(v) => v.get_step(),
            Payload::Ratification(r) => r.get_step(),
            Payload::Quorum(_) => {
                // TODO: This should be removed in future
                StepName::Ratification.to_step(self.header.iteration)
            }
            _ => StepName::Proposal.to_step(self.header.iteration),
        }
    }

    pub fn get_iteration(&self) -> u8 {
        self.header.iteration
    }

    pub fn get_height(&self) -> u64 {
        self.header.round
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn ray_id(&self) -> &str {
        self.metadata
            .as_ref()
            .map(|m| m.ray_id.as_str())
            .unwrap_or_default()
    }

    pub fn with_version(mut self, v: Version) -> Self {
        self.version = v;
        self
    }

    pub fn is_local(&self) -> bool {
        self.metadata.is_none()
    }
}

/// Defines a transport-related properties that determines how the message
/// will be broadcast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub height: u8,
    pub src_addr: SocketAddr,
    pub ray_id: String,
}

impl Serializable for Message {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.version.write(w)?;
        w.write_all(&[self.topic as u8])?;

        match &self.payload {
            Payload::Candidate(p) => p.write(w),
            Payload::Validation(p) => p.write(w),
            Payload::Quorum(p) => p.write(w),
            Payload::Block(p) => p.write(w),
            Payload::Transaction(p) => p.write(w),
            Payload::GetMempool(p) => p.write(w),
            Payload::Inv(p) => p.write(w),
            Payload::GetBlocks(p) => p.write(w),
            Payload::GetResource(p) => p.write(w),
            Payload::Ratification(p) => p.write(w),
            Payload::ValidationQuorum(p) => p.write(w),
            Payload::Empty | Payload::ValidationResult(_) => Ok(()), /* internal message, not sent on the wire */
        }
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let version = Version::read(r)?;

        // Read topic
        let topic = Topics::from(Self::read_u8(r)?);
        let message: Message = match topic {
            Topics::Candidate => payload::Candidate::read(r)?.into(),
            Topics::Validation => payload::Validation::read(r)?.into(),
            Topics::Ratification => payload::Ratification::read(r)?.into(),
            Topics::Quorum => payload::Quorum::read(r)?.into(),
            Topics::ValidationQuorum => {
                payload::ValidationQuorum::read(r)?.into()
            }
            Topics::Block => ledger::Block::read(r)?.into(),
            Topics::Tx => ledger::Transaction::read(r)?.into(),
            Topics::GetResource => payload::GetResource::read(r)?.into(),
            Topics::GetBlocks => payload::GetBlocks::read(r)?.into(),
            Topics::GetMempool => payload::GetMempool::read(r)?.into(),
            Topics::Inv => payload::Inv::read(r)?.into(),
            Topics::Unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unknown topic",
                ));
            }
        };

        Ok(message.with_version(version))
    }
}

impl<W: WireMessage> From<W> for Message {
    fn from(wire_msg: W) -> Self {
        Self {
            header: wire_msg.consensus_header(),
            topic: W::TOPIC,
            payload: wire_msg.payload(),
            ..Default::default()
        }
    }
}

impl WireMessage for Candidate {
    const TOPIC: Topics = Topics::Candidate;
    fn consensus_header(&self) -> ConsensusHeader {
        self.header()
    }
}

impl WireMessage for Validation {
    const TOPIC: Topics = Topics::Validation;
    fn consensus_header(&self) -> ConsensusHeader {
        self.header
    }
}

impl WireMessage for Ratification {
    const TOPIC: Topics = Topics::Ratification;
    fn consensus_header(&self) -> ConsensusHeader {
        self.header
    }
}

impl WireMessage for payload::Quorum {
    const TOPIC: Topics = Topics::Quorum;
    fn consensus_header(&self) -> ConsensusHeader {
        self.header
    }
}

impl WireMessage for payload::GetMempool {
    const TOPIC: Topics = Topics::GetMempool;
}

impl WireMessage for payload::Inv {
    const TOPIC: Topics = Topics::Inv;
}

impl WireMessage for payload::GetBlocks {
    const TOPIC: Topics = Topics::GetBlocks;
}

impl WireMessage for payload::GetResource {
    const TOPIC: Topics = Topics::GetResource;
}

impl WireMessage for ledger::Block {
    const TOPIC: Topics = Topics::Block;
}

impl WireMessage for ledger::Transaction {
    const TOPIC: Topics = Topics::Tx;
}

impl WireMessage for payload::ValidationQuorum {
    const TOPIC: Topics = Topics::ValidationQuorum;
}

impl WireMessage for payload::ValidationResult {
    const TOPIC: Topics = Topics::Unknown;
}

impl Message {
    /// Creates a unknown message with empty payload
    pub fn empty() -> Message {
        Self {
            topic: Topics::default(),
            payload: Payload::Empty,
            ..Default::default()
        }
    }

    pub fn topic(&self) -> Topics {
        self.topic
    }
}

#[derive(Default, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
pub struct ConsensusHeader {
    pub prev_block_hash: Hash,
    pub round: u64,
    #[cfg_attr(any(feature = "faker", test), dummy(faker = "0..50"))]
    pub iteration: u8,
}

impl std::fmt::Debug for ConsensusHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsensusHeader")
            .field("prev_block_hash", &to_str(&self.prev_block_hash))
            .field("round", &self.round)
            .field("iteration", &self.iteration)
            .finish()
    }
}

impl Serializable for ConsensusHeader {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.prev_block_hash)?;
        w.write_all(&self.round.to_le_bytes())?;
        w.write_all(&[self.iteration])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let prev_block_hash = Self::read_bytes(r)?;
        let round = Self::read_u64_le(r)?;
        let iteration = Self::read_u8(r)?;

        // Iteration is 0-based
        if iteration >= MESSAGE_MAX_ITER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid iteration {iteration})"),
            ));
        }

        Ok(ConsensusHeader {
            prev_block_hash,
            round,
            iteration,
        })
    }
}

impl ConsensusHeader {
    pub fn compare_round(&self, round: u64) -> Status {
        if self.round == round {
            return Status::Present;
        }

        if self.round > round {
            return Status::Future;
        }

        Status::Past
    }

    pub fn signable(&self) -> Vec<u8> {
        let mut buf = vec![];
        self.write(&mut buf).expect("Writing to vec should succeed");
        buf
    }
}

#[derive(Default, Debug, Clone)]
pub enum Payload {
    Ratification(payload::Ratification),
    Validation(payload::Validation),
    Candidate(Box<payload::Candidate>),
    Quorum(payload::Quorum),
    ValidationQuorum(Box<payload::ValidationQuorum>),

    Block(Box<ledger::Block>),
    Transaction(Box<ledger::Transaction>),
    GetMempool(payload::GetMempool),
    Inv(payload::Inv),
    GetBlocks(payload::GetBlocks),
    GetResource(payload::GetResource),

    // Internal messages payload
    // Result message passed from Validation step to Ratification step
    ValidationResult(Box<payload::ValidationResult>),

    #[default]
    Empty,
}

impl Payload {
    pub fn set_nonce<N: Into<Nonce>>(&mut self, nonce: N) {
        match self {
            Payload::GetMempool(p) => p.set_nonce(nonce),
            Payload::GetBlocks(p) => p.set_nonce(nonce),
            _ => {}
        }
    }
}

impl From<payload::Ratification> for Payload {
    fn from(value: payload::Ratification) -> Self {
        Self::Ratification(value)
    }
}

impl From<payload::Validation> for Payload {
    fn from(value: payload::Validation) -> Self {
        Self::Validation(value)
    }
}

impl From<payload::Candidate> for Payload {
    fn from(value: payload::Candidate) -> Self {
        Self::Candidate(Box::new(value))
    }
}
impl From<payload::Quorum> for Payload {
    fn from(value: payload::Quorum) -> Self {
        Self::Quorum(value)
    }
}
impl From<ledger::Block> for Payload {
    fn from(value: ledger::Block) -> Self {
        Self::Block(Box::new(value))
    }
}
impl From<ledger::Transaction> for Payload {
    fn from(value: ledger::Transaction) -> Self {
        Self::Transaction(Box::new(value))
    }
}
impl From<payload::GetMempool> for Payload {
    fn from(value: payload::GetMempool) -> Self {
        Self::GetMempool(value)
    }
}
impl From<payload::Inv> for Payload {
    fn from(value: payload::Inv) -> Self {
        Self::Inv(value)
    }
}
impl From<payload::GetBlocks> for Payload {
    fn from(value: payload::GetBlocks) -> Self {
        Self::GetBlocks(value)
    }
}
impl From<payload::GetResource> for Payload {
    fn from(value: payload::GetResource) -> Self {
        Self::GetResource(value)
    }
}

impl From<payload::ValidationQuorum> for Payload {
    fn from(value: payload::ValidationQuorum) -> Self {
        Self::ValidationQuorum(Box::new(value))
    }
}

impl From<payload::ValidationResult> for Payload {
    fn from(value: payload::ValidationResult) -> Self {
        Self::ValidationResult(Box::new(value))
    }
}

pub mod payload {
    use crate::ledger::{self, to_str, Attestation, Block, Hash, StepVotes};
    use crate::{get_current_timestamp, Serializable};
    use std::fmt;
    use std::io::{self, Read, Write};
    use std::net::{
        IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
    };

    use super::{ConsensusHeader, SignInfo};
    use serde::Serialize;

    #[derive(Debug, Clone)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct Ratification {
        pub header: ConsensusHeader,
        pub vote: Vote,
        pub timestamp: u64,
        pub validation_result: ValidationResult,
        pub sign_info: SignInfo,
    }

    #[derive(Debug, Clone)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct Validation {
        pub header: ConsensusHeader,
        pub vote: Vote,
        pub sign_info: SignInfo,
    }

    #[derive(
        Clone, Copy, Hash, Eq, PartialEq, Default, PartialOrd, Ord, Serialize,
    )]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    #[repr(u8)]
    pub enum Vote {
        NoCandidate = 0,
        Valid(#[serde(serialize_with = "crate::serialize_hex")] Hash) = 1,
        Invalid(#[serde(serialize_with = "crate::serialize_hex")] Hash) = 2,

        #[default]
        NoQuorum = 3,
    }

    impl Vote {
        pub fn is_valid(&self) -> bool {
            matches!(self, Vote::Valid(_))
        }
        pub fn size(&self) -> usize {
            const ENUM_BYTE: usize = 1;

            let data_size: usize = match &self {
                Vote::NoCandidate => 0,
                Vote::Valid(_) => 32,
                Vote::Invalid(_) => 32,
                Vote::NoQuorum => 0,
            };
            ENUM_BYTE + data_size
        }
    }

    impl fmt::Debug for Vote {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let (desc, hash) = match &self {
                Self::NoCandidate => ("NoCandidate", "".into()),
                Self::Valid(hash) => ("Valid", to_str(hash)),
                Self::Invalid(hash) => ("Invalid", to_str(hash)),
                Self::NoQuorum => ("NoQuorum", "".into()),
            };
            write!(f, "Vote: {desc}({hash})")
        }
    }

    impl Serializable for Vote {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            match &self {
                Self::NoCandidate => w.write_all(&[0])?,

                Self::Valid(hash) => {
                    w.write_all(&[1])?;
                    w.write_all(hash)?;
                }
                Self::Invalid(hash) => {
                    w.write_all(&[2])?;
                    w.write_all(hash)?;
                }
                Self::NoQuorum => w.write_all(&[3])?,
            };
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(match Self::read_u8(r)? {
                0 => Self::NoCandidate,
                1 => Self::Valid(Self::read_bytes(r)?),
                2 => Self::Invalid(Self::read_bytes(r)?),
                3 => Self::NoQuorum,
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid vote",
                ))?,
            })
        }
    }

    impl Serializable for Validation {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.header.write(w)?;
            self.vote.write(w)?;
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
            let sign_info = SignInfo::read(r)?;

            Ok(Validation {
                header,
                vote,
                sign_info,
            })
        }
    }

    #[derive(Clone)]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    pub struct Candidate {
        pub candidate: Block,
    }

    impl std::fmt::Debug for Candidate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Candidate")
                .field(
                    "signature",
                    &ledger::to_str(self.candidate.header().signature.inner()),
                )
                .field("block", &self.candidate)
                .finish()
        }
    }

    impl PartialEq<Self> for Candidate {
        fn eq(&self, other: &Self) -> bool {
            self.candidate
                .header()
                .hash
                .eq(&other.candidate.header().hash)
        }
    }

    impl Eq for Candidate {}

    impl Serializable for Candidate {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.candidate.write(w)?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let candidate = Block::read(r)?;

            Ok(Candidate { candidate })
        }
    }
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    pub enum QuorumType {
        /// Supermajority of Valid votes
        Valid = 0,
        /// Majority of Invalid votes
        Invalid = 1,
        /// Majority of NoCandidate votes
        NoCandidate = 2,
        /// No quorum reached (timeout expired)
        #[default]
        NoQuorum = 255,
    }

    impl From<u8> for QuorumType {
        fn from(v: u8) -> QuorumType {
            match v {
                0 => QuorumType::Valid,
                1 => QuorumType::Invalid,
                2 => QuorumType::NoCandidate,
                _ => QuorumType::NoQuorum,
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct ValidationQuorum {
        pub header: ConsensusHeader,
        pub result: ValidationResult,
    }

    #[derive(Debug, Clone, Default)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct ValidationResult {
        pub(crate) quorum: QuorumType,
        pub(crate) vote: Vote,
        pub(crate) sv: StepVotes,
    }

    impl ValidationResult {
        pub fn new(sv: StepVotes, vote: Vote, quorum: QuorumType) -> Self {
            Self { sv, vote, quorum }
        }

        pub fn quorum(&self) -> QuorumType {
            self.quorum
        }

        pub fn sv(&self) -> &StepVotes {
            &self.sv
        }

        pub fn vote(&self) -> &Vote {
            &self.vote
        }
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    pub enum RatificationResult {
        Fail(Vote),
        Success(Vote),
    }

    impl Default for RatificationResult {
        fn default() -> Self {
            Self::Fail(Vote::NoQuorum)
        }
    }

    impl From<Vote> for RatificationResult {
        fn from(vote: Vote) -> Self {
            match vote {
                Vote::Valid(hash) => {
                    RatificationResult::Success(Vote::Valid(hash))
                }
                fail => RatificationResult::Fail(fail),
            }
        }
    }

    impl RatificationResult {
        pub fn vote(&self) -> &Vote {
            match self {
                Self::Success(v) => v,
                Self::Fail(v) => v,
            }
        }

        pub fn failed(&self) -> bool {
            match self {
                Self::Success(_) => false,
                Self::Fail(_) => true,
            }
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Quorum {
        pub header: ConsensusHeader,
        pub att: Attestation,
    }

    impl Serializable for Quorum {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.header.write(w)?;
            self.att.write(w)?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let header = ConsensusHeader::read(r)?;
            let att = Attestation::read(r)?;

            Ok(Quorum { header, att })
        }
    }

    impl Quorum {
        pub fn vote(&self) -> &Vote {
            self.att.result.vote()
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetCandidate {
        pub hash: [u8; 32],
    }

    impl Serializable for GetCandidate {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.hash[..])?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let hash = Self::read_bytes(r)?;

            Ok(GetCandidate { hash })
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetCandidateResp {
        pub candidate: Block,
    }

    impl Serializable for GetCandidateResp {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.candidate.write(w)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(GetCandidateResp {
                candidate: Block::read(r)?,
            })
        }
    }
    #[derive(Debug, Clone, Default)]
    pub struct Nonce([u8; 8]);

    impl Serializable for Nonce {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.0)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let nonce = Self::read_bytes(r)?;
            Ok(Self(nonce))
        }
    }

    impl From<Nonce> for u64 {
        fn from(value: Nonce) -> Self {
            u64::from_le_bytes(value.0)
        }
    }

    impl From<u64> for Nonce {
        fn from(value: u64) -> Self {
            Self(value.to_le_bytes())
        }
    }

    impl From<IpAddr> for Nonce {
        fn from(value: IpAddr) -> Self {
            match value {
                IpAddr::V4(v4) => v4.into(),
                IpAddr::V6(v6) => v6.into(),
            }
        }
    }

    impl From<Ipv4Addr> for Nonce {
        fn from(value: Ipv4Addr) -> Self {
            let num = u32::from_le_bytes(value.octets());
            (num as u64).into()
        }
    }

    impl From<Ipv6Addr> for Nonce {
        fn from(value: Ipv6Addr) -> Self {
            let mut ret = [0u8; 8];
            let value = value.octets();
            let (a, b) = value.split_at(8);
            a.iter()
                .zip(b)
                .map(|(a, b)| a.wrapping_add(*b))
                .enumerate()
                .for_each(|(idx, v)| ret[idx] = v);

            Self(ret)
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetMempool {
        pub(crate) nonce: Nonce,
    }

    impl GetMempool {
        pub fn set_nonce<N: Into<Nonce>>(&mut self, nonce: N) {
            self.nonce = nonce.into()
        }
    }

    impl Serializable for GetMempool {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.nonce.write(w)?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let nonce = Nonce::read(r)?;
            Ok(GetMempool { nonce })
        }
    }

    #[derive(Clone, Default, Debug, Copy)]
    pub enum InvType {
        /// A transaction fetched by tx_id
        MempoolTx,
        #[default]
        /// A full block fetched by block hash
        BlockFromHash,
        /// A full block fetched by block height
        BlockFromHeight,
        /// A candidate block fetched by block hash, Att is None
        CandidateFromHash,
        /// A candidate block fetched by (prev_block_hash, iteration)
        CandidateFromIteration,
        /// A ValidationResult fetched by (prev_block_hash, round, iteration)
        ValidationResult,
    }

    #[derive(Clone, Copy)]
    pub enum InvParam {
        Hash([u8; 32]),
        Height(u64),
        HashAndIteration([u8; 32], u8),
        Iteration(ConsensusHeader),
    }

    impl Default for InvParam {
        fn default() -> Self {
            Self::Height(0)
        }
    }

    impl fmt::Debug for InvParam {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                InvParam::Hash(hash) => write!(f, "Hash: {}", to_str(hash)),
                InvParam::Height(height) => write!(f, "Height: {}", height),
                InvParam::HashAndIteration(hash, iteration) => {
                    write!(
                        f,
                        "Hash: {}, Iteration: {}",
                        to_str(hash),
                        iteration
                    )
                }
                InvParam::Iteration(ch) => {
                    write!(
                        f,
                        "PrevBlock: {}, Round: {}, Iteration: {}",
                        to_str(&ch.prev_block_hash),
                        ch.round,
                        ch.iteration
                    )
                }
            }
        }
    }

    #[derive(Default, Debug, Clone, Copy)]
    pub struct InvVect {
        pub inv_type: InvType,
        pub param: InvParam,
    }

    #[derive(Default, Debug, Clone)]
    pub struct Inv {
        pub inv_list: Vec<InvVect>,
        pub max_entries: u16,
    }

    impl Inv {
        pub fn new(max_entries: u16) -> Self {
            Self {
                inv_list: Default::default(),
                max_entries,
            }
        }

        pub fn add_tx_id(&mut self, id: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::MempoolTx,
                param: InvParam::Hash(id),
            });
        }

        pub fn add_block_from_hash(&mut self, hash: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::BlockFromHash,
                param: InvParam::Hash(hash),
            });
        }

        pub fn add_block_from_height(&mut self, height: u64) {
            self.inv_list.push(InvVect {
                inv_type: InvType::BlockFromHeight,
                param: InvParam::Height(height),
            });
        }

        pub fn add_candidate_from_hash(&mut self, hash: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::CandidateFromHash,
                param: InvParam::Hash(hash),
            });
        }

        pub fn add_candidate_from_iteration(
            &mut self,
            prev_block_hash: [u8; 32],
            iteration: u8,
        ) {
            self.inv_list.push(InvVect {
                inv_type: InvType::CandidateFromIteration,
                param: InvParam::HashAndIteration(prev_block_hash, iteration),
            });
        }

        pub fn add_validation_result(
            &mut self,
            consensus_header: ConsensusHeader,
        ) {
            self.inv_list.push(InvVect {
                inv_type: InvType::ValidationResult,
                param: InvParam::Iteration(consensus_header),
            });
        }
    }

    impl Serializable for Inv {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            let items_len = self.inv_list.len() as u32;
            w.write_all(&items_len.to_le_bytes())?;

            for item in &self.inv_list {
                w.write_all(&[item.inv_type as u8])?;

                match &item.param {
                    InvParam::Hash(hash) => w.write_all(&hash[..])?,
                    InvParam::Height(height) => {
                        w.write_all(&height.to_le_bytes())?
                    }
                    InvParam::HashAndIteration(hash, iteration) => {
                        w.write_all(&hash[..])?;
                        w.write_all(&[*iteration])?;
                    }
                    InvParam::Iteration(ch) => {
                        ch.write(w)?;
                    }
                };
            }

            w.write_all(&self.max_entries.to_le_bytes())?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let items_len = Self::read_u32_le(r)?;

            let mut inv = Inv::default();
            for _ in 0..items_len {
                let inv_type = Self::read_u8(r)?;

                let inv_type = match inv_type {
                    0 => InvType::MempoolTx,
                    1 => InvType::BlockFromHash,
                    2 => InvType::BlockFromHeight,
                    3 => InvType::CandidateFromHash,
                    4 => InvType::CandidateFromIteration,
                    5 => InvType::ValidationResult,
                    _ => {
                        return Err(io::Error::from(io::ErrorKind::InvalidData))
                    }
                };

                match inv_type {
                    InvType::MempoolTx => {
                        let hash = Self::read_bytes(r)?;
                        inv.add_tx_id(hash);
                    }
                    InvType::BlockFromHash => {
                        let hash = Self::read_bytes(r)?;
                        inv.add_block_from_hash(hash);
                    }
                    InvType::BlockFromHeight => {
                        inv.add_block_from_height(Self::read_u64_le(r)?);
                    }
                    InvType::CandidateFromHash => {
                        inv.add_candidate_from_hash(Self::read_bytes(r)?);
                    }
                    InvType::CandidateFromIteration => {
                        let prev_block_hash = Self::read_bytes(r)?;
                        let iteration = Self::read_u8(r)?;
                        inv.add_candidate_from_iteration(
                            prev_block_hash,
                            iteration,
                        );
                    }
                    InvType::ValidationResult => {
                        let ch = ConsensusHeader::read(r)?;
                        inv.add_validation_result(ch);
                    }
                }
            }

            inv.max_entries = Self::read_u16_le(r)?;
            Ok(inv)
        }
    }

    #[derive(Clone)]
    pub struct GetBlocks {
        pub locator: [u8; 32],
        pub(crate) nonce: Nonce,
    }

    impl GetBlocks {
        pub fn new(locator: [u8; 32]) -> Self {
            Self {
                locator,
                nonce: Nonce::default(),
            }
        }
        pub fn set_nonce<N: Into<Nonce>>(&mut self, nonce: N) {
            self.nonce = nonce.into()
        }
    }

    impl fmt::Debug for GetBlocks {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "GetBlocks, locator: {}", to_str(&self.locator))
        }
    }

    impl Serializable for GetBlocks {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.locator[..])?;
            self.nonce.write(w)?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let locator = Self::read_bytes(r)?;
            let nonce = Nonce::read(r)?;
            Ok(Self { locator, nonce })
        }
    }

    #[derive(Debug, Clone)]
    pub struct GetResource {
        /// Inventory/Resource to search for
        inventory: Inv,

        /// (requester) Address to which the resource is sent back, if found
        requester_addr: Option<SocketAddr>,

        /// Limits request lifespan by absolute (epoch) time
        ttl_as_sec: u64,

        /// Limits request lifespan by number of hops
        hops_limit: u16,
    }

    impl GetResource {
        pub fn new(
            inventory: Inv,
            requester_addr: Option<SocketAddr>,
            ttl_as_sec: u64,
            hops_limit: u16,
        ) -> Self {
            Self {
                inventory,
                requester_addr,
                ttl_as_sec,
                hops_limit,
            }
        }

        pub fn clone_with_hop_decrement(&self) -> Option<Self> {
            if self.hops_limit <= 1 {
                return None;
            }
            let mut req = self.clone();
            req.hops_limit -= 1;
            Some(req)
        }

        pub fn get_addr(&self) -> Option<SocketAddr> {
            self.requester_addr
        }

        pub fn get_inv(&self) -> &Inv {
            &self.inventory
        }

        pub fn is_expired(&self) -> bool {
            get_current_timestamp() > self.ttl_as_sec
        }
    }

    impl Serializable for GetResource {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.inventory.write(w)?;

            let requester_addr = self.requester_addr.ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "Requester address is missing",
            ))?;

            requester_addr.write(w)?;
            w.write_all(&self.ttl_as_sec.to_le_bytes()[..])?;
            w.write_all(&self.hops_limit.to_le_bytes()[..])
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let inner = Inv::read(r)?;
            let requester_addr = SocketAddr::read(r)?;

            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;
            let ttl_as_sec = u64::from_le_bytes(buf);

            let mut buf = [0u8; 2];
            r.read_exact(&mut buf)?;
            let hops_limit = u16::from_le_bytes(buf);

            Ok(GetResource {
                inventory: inner,
                requester_addr: Some(requester_addr),
                ttl_as_sec,
                hops_limit,
            })
        }
    }

    impl Serializable for SocketAddr {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            match self {
                SocketAddr::V4(addr_v4) => {
                    w.write_all(&[4])?;
                    w.write_all(&addr_v4.ip().octets())?;
                    w.write_all(&addr_v4.port().to_le_bytes())?;
                }
                SocketAddr::V6(addr_v6) => {
                    w.write_all(&[6])?;
                    w.write_all(&addr_v6.ip().octets())?;
                    w.write_all(&addr_v6.port().to_le_bytes())?;
                }
            }
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut ip_type = [0u8; 1];
            r.read_exact(&mut ip_type)?;

            let ip = match ip_type[0] {
                4 => {
                    let mut octets = [0u8; 4];
                    r.read_exact(&mut octets)?;

                    let mut port_bytes = [0u8; 2];
                    r.read_exact(&mut port_bytes)?;

                    SocketAddr::V4(SocketAddrV4::new(
                        Ipv4Addr::from(octets),
                        u16::from_le_bytes(port_bytes),
                    ))
                }
                6 => {
                    let mut octets = [0u8; 16];
                    r.read_exact(&mut octets)?;

                    let mut port_bytes = [0u8; 2];
                    r.read_exact(&mut port_bytes)?;

                    SocketAddr::V6(SocketAddrV6::new(
                        Ipv6Addr::from(octets),
                        u16::from_le_bytes(port_bytes),
                        0,
                        0,
                    ))
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid IP type",
                    ))
                }
            };
            Ok(ip)
        }
    }
}

macro_rules! map_topic {
    ($v:expr, $enum_v:expr) => {
        if $v == $enum_v as u8 {
            return $enum_v;
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
pub enum Topics {
    // Data exchange topics.
    GetResource = 8,
    GetBlocks = 9,
    GetMempool = 13, // NB: This is aliased as Mempool in the golang impl
    Inv = 14,

    // Fire-and-forget messaging
    Tx = 10,
    Block = 11,

    // Consensus main loop topics
    Candidate = 16,
    Validation = 17,
    Ratification = 18,
    Quorum = 19,
    ValidationQuorum = 20,

    #[default]
    Unknown = 255,
}

impl From<u8> for Topics {
    fn from(v: u8) -> Self {
        map_topic!(v, Topics::GetResource);
        map_topic!(v, Topics::GetBlocks);
        map_topic!(v, Topics::Tx);
        map_topic!(v, Topics::Block);
        map_topic!(v, Topics::GetMempool);
        map_topic!(v, Topics::Inv);
        map_topic!(v, Topics::Candidate);
        map_topic!(v, Topics::Validation);
        map_topic!(v, Topics::Ratification);
        map_topic!(v, Topics::Quorum);

        Topics::Unknown
    }
}

impl From<Topics> for u8 {
    fn from(t: Topics) -> Self {
        t as u8
    }
}

/// AsyncQueue is a thin wrapper of async_channel.
#[derive(Clone)]
pub struct AsyncQueue<M: Clone> {
    receiver: async_channel::Receiver<M>,
    sender: async_channel::Sender<M>,

    cap: usize,
    label: &'static str,
}

impl<M: Clone> AsyncQueue<M> {
    /// Creates a bounded async queue with fixed capacity
    ///
    /// `Label` sets a queue label for logging
    ///
    /// Panics if `cap` is zero (Capacity must be a positive number).
    pub fn bounded(cap: usize, label: &'static str) -> Self {
        let (sender, receiver) = async_channel::bounded(cap);
        Self {
            receiver,
            sender,
            cap,
            label,
        }
    }
}

impl<M: Clone> AsyncQueue<M> {
    pub fn try_send(&self, msg: M) {
        let label = self.label;
        let _ = self.sender.try_send(msg).map_err(|err| match err {
            TrySendError::Full(_) => {
                error!("queue ({label}) is full, cap: {}", self.cap);
            }
            TrySendError::Closed(_) => {
                error!("queue ({label}) is closed");
            }
        });
    }

    pub fn recv(&self) -> async_channel::Recv<'_, M> {
        self.receiver.recv()
    }
}

pub trait StepMessage {
    const STEP_NAME: StepName;
    fn header(&self) -> ConsensusHeader;

    fn get_step(&self) -> u8 {
        Self::STEP_NAME.to_step(self.header().iteration)
    }
}

pub trait SignedStepMessage: StepMessage {
    const SIGN_SEED: &'static [u8];
    fn signable(&self) -> Vec<u8>;
    fn sign_info(&self) -> SignInfo;
    fn sign_info_mut(&mut self) -> &mut SignInfo;

    fn verify_signature(&self) -> Result<(), BlsSigError> {
        let signature = self.sign_info().signature;
        let sig = BlsMultisigSignature::from_bytes(signature.inner())?;
        let pk = BlsMultisigPublicKey::aggregate(&[*self
            .sign_info()
            .signer
            .inner()])?;
        let msg = self.signable();
        pk.verify(&sig, &msg)
    }

    fn sign(&mut self, sk: &BlsSecretKey, pk: &BlsPublicKey) {
        let msg = self.signable();
        let sign_info = self.sign_info_mut();
        let signature = sk.sign_multisig(pk, &msg).to_bytes();
        sign_info.signature = signature.into();
        sign_info.signer = PublicKey::new(*pk)
    }
}

impl StepMessage for Validation {
    const STEP_NAME: StepName = StepName::Validation;

    fn header(&self) -> ConsensusHeader {
        self.header
    }
}

impl SignedStepMessage for Validation {
    const SIGN_SEED: &'static [u8] = &[1u8];

    fn sign_info(&self) -> SignInfo {
        self.sign_info.clone()
    }
    fn sign_info_mut(&mut self) -> &mut SignInfo {
        &mut self.sign_info
    }
    fn signable(&self) -> Vec<u8> {
        let mut signable = self.header.signable();
        signable.extend_from_slice(Self::SIGN_SEED);
        self.vote
            .write(&mut signable)
            .expect("Writing to vec should succeed");
        signable
    }
}

impl StepMessage for Ratification {
    const STEP_NAME: StepName = StepName::Ratification;

    fn header(&self) -> ConsensusHeader {
        self.header
    }
}

impl SignedStepMessage for Ratification {
    const SIGN_SEED: &'static [u8] = &[2u8];
    fn sign_info(&self) -> SignInfo {
        self.sign_info.clone()
    }
    fn sign_info_mut(&mut self) -> &mut SignInfo {
        &mut self.sign_info
    }
    fn signable(&self) -> Vec<u8> {
        let mut signable = self.header.signable();
        signable.extend_from_slice(Self::SIGN_SEED);
        self.vote
            .write(&mut signable)
            .expect("Writing to vec should succeed");
        signable
    }
}

impl StepMessage for Candidate {
    const STEP_NAME: StepName = StepName::Proposal;

    fn header(&self) -> ConsensusHeader {
        ConsensusHeader {
            iteration: self.candidate.header().iteration,
            prev_block_hash: self.candidate.header().prev_block_hash,
            round: self.candidate.header().height,
        }
    }
}

impl SignedStepMessage for Candidate {
    const SIGN_SEED: &'static [u8] = &[];
    fn sign_info(&self) -> SignInfo {
        let header = self.candidate.header();
        SignInfo {
            signer: PublicKey::try_from(header.generator_bls_pubkey.0)
                .unwrap_or_default(),
            signature: header.signature,
        }
    }
    fn sign_info_mut(&mut self) -> &mut SignInfo {
        panic!("sign_info_mut called on Candidate, this is a bug")
    }
    fn signable(&self) -> Vec<u8> {
        self.candidate.header().hash.to_vec()
    }

    fn sign(&mut self, sk: &BlsSecretKey, pk: &BlsPublicKey) {
        let msg = self.signable();
        let signature = sk.sign_multisig(pk, &msg).to_bytes();
        self.candidate.set_signature(signature.into());
    }
}

impl StepMessage for ValidationQuorum {
    const STEP_NAME: StepName = StepName::Validation;

    fn header(&self) -> ConsensusHeader {
        self.header
    }
}

#[derive(Clone, Default)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy, Eq, PartialEq))]
pub struct SignInfo {
    pub signer: bls::PublicKey,
    pub signature: Signature,
}

impl Serializable for SignInfo {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(self.signer.bytes().inner())?;
        w.write_all(self.signature.inner())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read bls pubkey
        let signer = Self::read_bytes(r)?;
        let signer = signer
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let signature = Self::read_bytes(r)?.into();

        Ok(Self { signer, signature })
    }
}

impl std::fmt::Debug for SignInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignInfo")
            .field("signer", &to_str(self.signature.inner()))
            .field("signature", &self.signature)
            .finish()
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use self::payload::ValidationResult;

    use super::*;
    use crate::ledger;
    use crate::ledger::*;
    use crate::Serializable;

    #[test]
    fn test_serialize() {
        let consensus_header = ConsensusHeader {
            iteration: 1,
            prev_block_hash: [2; 32],
            round: 4,
        };
        assert_serialize(consensus_header.clone());

        let header = ledger::Header {
            version: 3,
            height: 1888881,
            timestamp: 123456789,
            gas_limit: 111111111,
            prev_block_hash: [1; 32],
            seed: ledger::Seed::from([2; 48]),
            generator_bls_pubkey: bls::PublicKeyBytes([5; 96]),
            state_hash: [4; 32],
            event_bloom: [5; 256],
            hash: [6; 32],
            txroot: [7; 32],
            faultroot: [8; 32],
            att: Attestation {
                validation: ledger::StepVotes::new([6; 48], 22222222),
                ratification: ledger::StepVotes::new([7; 48], 3333333),
                ..Default::default()
            },
            iteration: 1,
            prev_block_cert: Attestation {
                validation: ledger::StepVotes::new([6; 48], 444444444),
                ratification: ledger::StepVotes::new([7; 48], 55555555),
                ..Default::default()
            },
            failed_iterations: Default::default(),
            signature: Signature::from([9; 48]),
        };

        let sample_block = ledger::Block::new(header, vec![], vec![])
            .expect("should be valid block");

        let sign_info = SignInfo {
            signer: bls::PublicKey::from_sk_seed_u64(3),
            signature: [5; 48].into(),
        };

        assert_serialize(payload::Candidate {
            candidate: sample_block,
        });

        assert_serialize(ledger::StepVotes::new([4; 48], 12345));

        assert_serialize(payload::Validation {
            header: consensus_header.clone(),
            vote: payload::Vote::Valid([4; 32]),
            sign_info: sign_info.clone(),
        });

        let validation_result = ValidationResult::new(
            ledger::StepVotes::new([1; 48], 12345),
            payload::Vote::Valid([5; 32]),
            payload::QuorumType::Valid,
        );

        assert_serialize(payload::Ratification {
            header: consensus_header.clone(),
            vote: payload::Vote::Valid([4; 32]),
            sign_info: sign_info.clone(),
            validation_result,
            timestamp: 1_000_000,
        });

        assert_serialize(payload::Quorum {
            header: consensus_header.clone(),
            att: Attestation {
                result: payload::Vote::Valid([4; 32]).into(),
                validation: ledger::StepVotes::new([1; 48], 12345),
                ratification: ledger::StepVotes::new([2; 48], 98765),
            },
        });
    }

    fn assert_serialize<S: Serializable + PartialEq + core::fmt::Debug>(v: S) {
        let mut buf = vec![];
        assert!(v.write(&mut buf).is_ok());
        let dup = S::read(&mut &buf[..]).expect("deserialize is ok");
        assert_eq!(
            v,
            dup,
            "failed to (de)serialize {}",
            std::any::type_name::<S>()
        );
    }
}
