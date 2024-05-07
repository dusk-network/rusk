// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable as DuskSerializable;
use tracing::warn;

use crate::bls::PublicKey;
use crate::ledger::{to_str, Hash, Signature};
use crate::StepName;
use crate::{bls, ledger, Serializable};
use std::cmp::Ordering;
use std::io::{self, Read, Write};
use std::net::SocketAddr;

use async_channel::TrySendError;

use self::payload::{Candidate, Ratification, Validation};

/// Topic field position in the message binary representation
pub const TOPIC_FIELD_POS: usize = 8 + 8 + 4;

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
    topic: Topics,
    pub header: ConsensusHeader,
    pub payload: Payload,

    pub metadata: Option<Metadata>,
}

impl Message {
    pub fn compare(&self, round: u64, iteration: u8, step: StepName) -> Status {
        self.header
            .round
            .cmp(&round)
            .then_with(|| self.get_step().cmp(&step.to_step(iteration)))
            .into()
    }
    pub fn get_signer(&self) -> Option<&bls::PublicKey> {
        let signer = match &self.payload {
            Payload::Candidate(c) => &c.sign_info().signer,
            Payload::Validation(v) => &v.sign_info().signer,
            Payload::Ratification(r) => &r.sign_info().signer,
            msg => {
                warn!("Calling get_signer for {msg:?}");
                return None;
            }
        };
        Some(signer)
    }
    pub fn get_step(&self) -> u16 {
        match &self.payload {
            Payload::Candidate(c) => c.get_step(),
            Payload::Validation(v) => v.get_step(),
            Payload::Ratification(r) => r.get_step(),
            Payload::Quorum(_) => {
                // This should be removed in future
                StepName::Ratification.to_step(self.header.iteration)
            }
            _ => StepName::Proposal.to_step(self.header.iteration),
        }
    }
}

/// Defines a transport-related properties that determines how the message
/// will be broadcast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub height: u8,
    pub src_addr: SocketAddr,
}

impl Serializable for Message {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&[self.topic as u8])?;

        match &self.payload {
            Payload::Candidate(p) => p.write(w),
            Payload::Validation(p) => p.write(w),
            Payload::Quorum(p) => p.write(w),
            Payload::Block(p) => p.write(w),
            Payload::Transaction(p) => p.write(w),
            Payload::GetCandidate(p) => p.write(w),
            Payload::CandidateResp(p) => p.write(w),
            Payload::GetMempool(p) => p.write(w),
            Payload::GetInv(p) => p.write(w),
            Payload::GetBlocks(p) => p.write(w),
            Payload::GetData(p) => p.write(w),
            Payload::Ratification(p) => p.write(w),
            Payload::Empty | Payload::ValidationResult(_) => Ok(()), /* internal message, not sent on the wire */
        }
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read topic
        let topic = Topics::from(Self::read_u8(r)?);
        let message = match topic {
            Topics::Candidate => {
                Message::new_candidate(payload::Candidate::read(r)?)
            }
            Topics::Validation => {
                Message::new_validation(payload::Validation::read(r)?)
            }
            Topics::Ratification => {
                Message::new_ratification(payload::Ratification::read(r)?)
            }
            Topics::Quorum => Message::new_quorum(payload::Quorum::read(r)?),
            Topics::Block => Message::new_block(ledger::Block::read(r)?),
            Topics::Tx => {
                Message::new_transaction(ledger::Transaction::read(r)?)
            }
            Topics::GetCandidateResp => Message::new_get_candidate_resp(
                payload::GetCandidateResp::read(r)?,
            ),
            Topics::GetCandidate => {
                Message::new_get_candidate(payload::GetCandidate::read(r)?)
            }
            Topics::GetData => {
                Message::new_get_data(payload::GetData::read(r)?)
            }
            Topics::GetBlocks => {
                Message::new_get_blocks(payload::GetBlocks::read(r)?)
            }
            Topics::GetMempool => {
                Message::new_get_mempool(payload::GetMempool::read(r)?)
            }
            Topics::GetInv => Message::new_inv(payload::Inv::read(r)?),
            Topics::Unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unknown topic",
                ));
            }
        };

        Ok(message)
    }
}

impl Message {
    /// Creates topics.Candidate message
    pub fn new_candidate(payload: payload::Candidate) -> Message {
        Self {
            header: payload.header.clone(),
            topic: Topics::Candidate,
            payload: Payload::Candidate(Box::new(payload)),
            ..Default::default()
        }
    }

    /// Creates topics.Ratification message
    pub fn new_ratification(payload: payload::Ratification) -> Message {
        Self {
            header: payload.header.clone(),
            topic: Topics::Ratification,
            payload: Payload::Ratification(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Validation message
    pub fn new_validation(payload: payload::Validation) -> Message {
        Self {
            header: payload.header.clone(),
            topic: Topics::Validation,
            payload: Payload::Validation(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Quorum message
    pub fn new_quorum(payload: payload::Quorum) -> Message {
        Self {
            header: payload.header.clone(),
            topic: Topics::Quorum,
            payload: Payload::Quorum(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Block message
    pub fn new_block(payload: ledger::Block) -> Message {
        Self {
            topic: Topics::Block,
            payload: Payload::Block(Box::new(payload)),
            ..Default::default()
        }
    }

    /// Creates topics.GetCandidate message
    pub fn new_get_candidate(p: payload::GetCandidate) -> Message {
        Self {
            topic: Topics::GetCandidate,
            payload: Payload::GetCandidate(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetCandidateResp message
    pub fn new_get_candidate_resp(p: payload::GetCandidateResp) -> Message {
        Self {
            topic: Topics::GetCandidateResp,
            payload: Payload::CandidateResp(Box::new(p)),
            ..Default::default()
        }
    }

    /// Creates topics.Inv (inventory) message
    pub fn new_inv(p: payload::Inv) -> Message {
        Self {
            topic: Topics::GetInv,
            payload: Payload::GetInv(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetData  message
    pub fn new_get_data(p: payload::GetData) -> Message {
        Self {
            topic: Topics::GetData,
            payload: Payload::GetData(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetMempool message
    pub fn new_get_mempool(p: payload::GetMempool) -> Message {
        Self {
            topic: Topics::GetMempool,
            payload: Payload::GetMempool(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetBlocks  message
    pub fn new_get_blocks(p: payload::GetBlocks) -> Message {
        Self {
            topic: Topics::GetBlocks,
            payload: Payload::GetBlocks(p),
            ..Default::default()
        }
    }

    /// Creates topics.Tx  message
    pub fn new_transaction(tx: ledger::Transaction) -> Message {
        Self {
            topic: Topics::Tx,
            payload: Payload::Transaction(Box::new(tx)),
            ..Default::default()
        }
    }

    /// Creates a message with a validation_result
    pub fn from_validation_result(p: payload::ValidationResult) -> Message {
        Self {
            topic: Topics::default(),
            payload: Payload::ValidationResult(Box::new(p)),
            ..Default::default()
        }
    }

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

#[derive(Default, Clone, PartialEq, Eq)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
pub struct ConsensusHeader {
    pub prev_block_hash: Hash,
    pub round: u64,
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

    Block(Box<ledger::Block>),
    Transaction(Box<ledger::Transaction>),
    GetCandidate(payload::GetCandidate),
    GetMempool(payload::GetMempool),
    GetInv(payload::Inv),
    GetBlocks(payload::GetBlocks),
    GetData(payload::GetData),
    CandidateResp(Box<payload::GetCandidateResp>),

    // Internal messages payload
    /// Result message passed from Validation step to Ratification step
    ValidationResult(Box<payload::ValidationResult>),

    #[default]
    Empty,
}

pub mod payload {
    use crate::ledger::{self, to_str, Block, Certificate, Hash, StepVotes};
    use crate::Serializable;
    use std::fmt;
    use std::io::{self, Read, Write};
    use std::net::{
        Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ConsensusHeader, SignInfo};

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

    #[derive(Clone, Copy, Hash, Eq, PartialEq, Default, PartialOrd, Ord)]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    #[repr(u8)]
    pub enum Vote {
        NoCandidate = 0,
        Valid(Hash) = 1,
        Invalid(Hash) = 2,

        #[default]
        NoQuorum = 3,
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
        pub header: ConsensusHeader,
        pub candidate: Block,
        pub sign_info: SignInfo,
    }

    impl std::fmt::Debug for Candidate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Candidate")
                .field(
                    "signature",
                    &ledger::to_str(self.sign_info.signature.inner()),
                )
                .field("block", &self.candidate)
                .finish()
        }
    }

    impl PartialEq<Self> for Candidate {
        fn eq(&self, other: &Self) -> bool {
            self.sign_info.signature.eq(&other.sign_info.signature)
                && self
                    .candidate
                    .header()
                    .hash
                    .eq(&other.candidate.header().hash)
        }
    }

    impl Eq for Candidate {}

    impl Serializable for Candidate {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.header.write(w)?;
            self.candidate.write(w)?;
            // sign_info at the end
            self.sign_info.write(w)?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let header = ConsensusHeader::read(r)?;
            let candidate = Block::read(r)?;
            let sign_info = SignInfo::read(r)?;

            Ok(Candidate {
                header,
                candidate,
                sign_info,
            })
        }
    }
    #[derive(Clone, Copy, Debug, Default)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
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

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
        pub cert: Certificate,
    }

    impl Serializable for Quorum {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.header.write(w)?;
            self.cert.write(w)?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let header = ConsensusHeader::read(r)?;
            let cert = Certificate::read(r)?;

            Ok(Quorum { header, cert })
        }
    }

    impl Quorum {
        pub fn vote(&self) -> &Vote {
            self.cert.result.vote()
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
    pub struct GetMempool {}

    impl Serializable for GetMempool {
        fn write<W: Write>(&self, _w: &mut W) -> io::Result<()> {
            Ok(())
        }

        fn read<R: Read>(_r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(GetMempool::default())
        }
    }

    #[derive(Clone, Default, Debug, Copy)]
    pub enum InvType {
        MempoolTx,
        #[default]
        BlockFromHash,
        BlockFromHeight,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum InvParam {
        Hash([u8; 32]),
        Height(u64),
    }

    impl Default for InvParam {
        fn default() -> Self {
            Self::Height(0)
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
    }

    impl Inv {
        pub fn add_tx_hash(&mut self, hash: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::MempoolTx,
                param: InvParam::Hash(hash),
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
                };
            }

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
                    _ => {
                        return Err(io::Error::from(io::ErrorKind::InvalidData))
                    }
                };

                match inv_type {
                    InvType::MempoolTx => {
                        let hash = Self::read_bytes(r)?;
                        inv.add_tx_hash(hash);
                    }
                    InvType::BlockFromHash => {
                        let hash = Self::read_bytes(r)?;
                        inv.add_block_from_hash(hash);
                    }
                    InvType::BlockFromHeight => {
                        inv.add_block_from_height(Self::read_u64_le(r)?);
                    }
                }
            }

            Ok(inv)
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetBlocks {
        pub locator: [u8; 32],
    }

    impl Serializable for GetBlocks {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.locator[..])
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let locator = Self::read_bytes(r)?;
            Ok(Self { locator })
        }
    }

    #[derive(Debug, Clone)]
    pub struct GetData {
        /// Inventory/Resource to search for
        inventory: Inv,

        /// (requester) Address to which the resource is sent back, if found
        requester_addr: SocketAddr,

        /// Request Time to live
        ttl_as_sec: u64,
        // TODO: Integrity test with hashing???
    }

    impl GetData {
        pub fn new(
            inventory: Inv,
            requester_addr: SocketAddr,
            ttl_as_sec: u64,
        ) -> Self {
            let ttl_as_sec = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + ttl_as_sec;

            Self {
                inventory,
                requester_addr,
                ttl_as_sec,
            }
        }

        pub fn get_addr(&self) -> SocketAddr {
            self.requester_addr
        }

        pub fn get_inv(&self) -> &Inv {
            &self.inventory
        }

        pub fn is_expired(&self) -> bool {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                > self.ttl_as_sec
        }
    }

    impl Serializable for GetData {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.inventory.write(w)?;
            self.requester_addr.write(w)?;
            w.write_all(&self.ttl_as_sec.to_le_bytes()[..])
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let inner = Inv::read(r)?;
            let requester_addr = SocketAddr::read(r)?;

            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;

            Ok(GetData {
                inventory: inner,
                requester_addr,
                ttl_as_sec: u64::from_le_bytes(buf),
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
    GetData = 8,
    GetBlocks = 9,
    GetMempool = 13, // NB: This is aliased as Mempool in the golang impl
    GetInv = 14,     // NB: This is aliased as Inv in the golang impl
    GetCandidate = 46,

    // Fire-and-forget messaging
    Tx = 10,
    Block = 11,

    // Consensus main loop topics
    GetCandidateResp = 15,
    Candidate = 16,
    Validation = 17,
    Ratification = 18,

    // Consensus Quorum loop topics
    Quorum = 19,

    #[default]
    Unknown = 255,
}

impl Topics {
    pub fn is_consensus_msg(&self) -> bool {
        matches!(
            &self,
            Topics::Candidate
                | Topics::Validation
                | Topics::Ratification
                | Topics::Quorum
        )
    }
}

impl From<u8> for Topics {
    fn from(v: u8) -> Self {
        map_topic!(v, Topics::GetData);
        map_topic!(v, Topics::GetBlocks);
        map_topic!(v, Topics::Tx);
        map_topic!(v, Topics::Block);
        map_topic!(v, Topics::GetMempool);
        map_topic!(v, Topics::GetInv);
        map_topic!(v, Topics::GetCandidateResp);
        map_topic!(v, Topics::GetCandidate);
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
}

impl<M: Clone> AsyncQueue<M> {
    pub fn bounded(cap: usize) -> Self {
        let (sender, receiver) = async_channel::bounded(cap);
        Self { receiver, sender }
    }

    pub fn unbounded() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self { receiver, sender }
    }
}

impl<M: Clone> AsyncQueue<M> {
    pub fn send(&self, msg: M) -> async_channel::Send<'_, M> {
        self.sender.send(msg)
    }

    pub fn try_send(&self, msg: M) -> Result<(), TrySendError<M>> {
        self.sender.try_send(msg)
    }

    pub fn recv(&self) -> async_channel::Recv<'_, M> {
        self.receiver.recv()
    }
}

pub trait StepMessage {
    const SIGN_SEED: &'static [u8];
    const STEP_NAME: StepName;
    fn signable(&self) -> Vec<u8>;
    fn header(&self) -> &ConsensusHeader;
    fn sign_info(&self) -> &SignInfo;
    fn sign_info_mut(&mut self) -> &mut SignInfo;

    fn get_step(&self) -> u16 {
        Self::STEP_NAME.to_step(self.header().iteration)
    }

    fn verify_signature(&self) -> Result<(), dusk_bls12_381_sign::Error> {
        let signature = self.sign_info().signature.inner();
        let sig = dusk_bls12_381_sign::Signature::from_bytes(signature)?;
        let pk =
            dusk_bls12_381_sign::APK::from(self.sign_info().signer.inner());
        let msg = self.signable();
        pk.verify(&sig, &msg)
    }

    fn sign(
        &mut self,
        sk: &dusk_bls12_381_sign::SecretKey,
        pk: &dusk_bls12_381_sign::PublicKey,
    ) {
        let msg = self.signable();
        let sign_info = self.sign_info_mut();
        let signature = sk.sign(pk, &msg).to_bytes();
        sign_info.signature = signature.into();
        sign_info.signer = PublicKey::new(*pk)
    }
}

impl StepMessage for Validation {
    const SIGN_SEED: &'static [u8] = &[1u8];
    const STEP_NAME: StepName = StepName::Validation;

    fn sign_info(&self) -> &SignInfo {
        &self.sign_info
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
    fn header(&self) -> &ConsensusHeader {
        &self.header
    }
}

impl StepMessage for Ratification {
    const SIGN_SEED: &'static [u8] = &[2u8];
    const STEP_NAME: StepName = StepName::Ratification;
    fn sign_info(&self) -> &SignInfo {
        &self.sign_info
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
    fn header(&self) -> &ConsensusHeader {
        &self.header
    }
}

impl StepMessage for Candidate {
    const SIGN_SEED: &'static [u8] = &[];
    const STEP_NAME: StepName = StepName::Proposal;
    fn sign_info(&self) -> &SignInfo {
        &self.sign_info
    }
    fn sign_info_mut(&mut self) -> &mut SignInfo {
        &mut self.sign_info
    }
    fn signable(&self) -> Vec<u8> {
        self.candidate.header().hash.to_vec()
    }
    fn header(&self) -> &ConsensusHeader {
        &self.header
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
            event_hash: [5; 32],
            hash: [6; 32],
            txroot: [7; 32],
            cert: Certificate {
                validation: ledger::StepVotes::new([6; 48], 22222222),
                ratification: ledger::StepVotes::new([7; 48], 3333333),
                ..Default::default()
            },
            iteration: 1,
            prev_block_cert: Certificate {
                validation: ledger::StepVotes::new([6; 48], 444444444),
                ratification: ledger::StepVotes::new([7; 48], 55555555),
                ..Default::default()
            },
            failed_iterations: Default::default(),
        };

        let sample_block =
            ledger::Block::new(header, vec![]).expect("should be valid block");

        let sign_info = SignInfo {
            signer: bls::PublicKey::from_sk_seed_u64(3),
            signature: [5; 48].into(),
        };

        assert_serialize(payload::Candidate {
            header: consensus_header.clone(),
            candidate: sample_block,
            sign_info: sign_info.clone(),
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
            cert: Certificate {
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
