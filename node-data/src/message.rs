// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::{Buf, BufMut, BytesMut};
use dusk_bytes::Serializable as DuskSerializable;

use crate::ledger::to_str;
use crate::StepName;
use crate::{bls, ledger, Serializable};
use std::cmp::Ordering;
use std::io::{self, Read, Write};
use std::net::SocketAddr;

use async_channel::TrySendError;

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

pub fn marshal_signable_vote(
    round: u64,
    step: u16,
    block_hash: &[u8; 32],
) -> BytesMut {
    const CAPACITY: usize = 32 + u64::SIZE + u16::SIZE;
    let mut msg = BytesMut::with_capacity(CAPACITY);
    msg.put_u64_le(round);
    msg.put_u16_le(step);
    msg.put(&block_hash[..]);

    msg
}

pub trait MessageTrait {
    fn compare(&self, round: u64, iteration: u8, step: StepName) -> Status;
    fn get_pubkey_bls(&self) -> &bls::PublicKey;
    fn get_block_hash(&self) -> [u8; 32];
    fn get_topic(&self) -> Topics;
    fn get_step(&self) -> u16;
}

/// Message definition
#[derive(Debug, Default, Clone)]
pub struct Message {
    pub header: Header,
    pub payload: Payload,

    pub metadata: Option<Metadata>,
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
        w.write_all(&[self.header.topic as u8])?;

        // Optional header fields used only for consensus messages
        if self.header.topic.is_consensus_msg() {
            self.header.write(w)?;
        }

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
        if topic == Topics::Unknown {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown topic",
            ));
        }

        // Decode message header only if the topic is supported
        let header = match topic.is_consensus_msg() {
            true => {
                let mut header = Header::read(r)?;
                header.topic = topic;
                header
            }
            false => Header::new(topic),
        };

        let payload = match topic {
            Topics::Candidate => {
                Payload::Candidate(Box::new(payload::Candidate::read(r)?))
            }
            Topics::Validation => {
                Payload::Validation(payload::Validation::read(r)?)
            }
            Topics::Ratification => {
                Payload::Ratification(payload::Ratification::read(r)?)
            }
            Topics::Quorum => Payload::Quorum(payload::Quorum::read(r)?),
            Topics::Block => Payload::Block(Box::new(ledger::Block::read(r)?)),
            Topics::Tx => {
                Payload::Transaction(Box::new(ledger::Transaction::read(r)?))
            }
            Topics::GetCandidateResp => Payload::CandidateResp(Box::new(
                payload::CandidateResp::read(r)?,
            )),
            Topics::GetCandidate => {
                Payload::GetCandidate(payload::GetCandidate::read(r)?)
            }
            Topics::GetData => Payload::GetData(payload::GetData::read(r)?),
            Topics::GetBlocks => {
                Payload::GetBlocks(payload::GetBlocks::read(r)?)
            }
            Topics::GetMempool => {
                Payload::GetMempool(payload::GetMempool::read(r)?)
            }
            Topics::GetInv => Payload::GetInv(payload::Inv::read(r)?),
            Topics::Unknown => Payload::Empty,
        };

        Ok(Message {
            header,
            payload,
            metadata: Default::default(),
        })
    }
}

impl MessageTrait for Message {
    fn compare(&self, round: u64, iteration: u8, step: StepName) -> Status {
        self.header.compare(round, iteration, step)
    }
    fn get_pubkey_bls(&self) -> &bls::PublicKey {
        &self.header.pubkey_bls
    }
    fn get_block_hash(&self) -> [u8; 32] {
        self.header.block_hash
    }
    fn get_topic(&self) -> Topics {
        self.header.topic
    }

    fn get_step(&self) -> u16 {
        self.header.get_step()
    }
}

impl Header {
    pub fn get_step(&self) -> u16 {
        let step = self.iteration as u16 * 3;
        match self.topic {
            Topics::Validation => step + 1,
            Topics::Ratification | Topics::Quorum => step + 2,
            _ => step,
        }
    }
}

impl Message {
    /// Creates topics.NewBlock message
    pub fn new_newblock(header: Header, p: payload::Candidate) -> Message {
        Self {
            header,
            payload: Payload::Candidate(Box::new(p)),
            ..Default::default()
        }
    }

    /// Creates topics.Ratification message
    pub fn new_ratification(
        header: Header,
        payload: payload::Ratification,
    ) -> Message {
        Self {
            header,
            payload: Payload::Ratification(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Validation message
    pub fn new_validation(
        header: Header,
        payload: payload::Validation,
    ) -> Message {
        Self {
            header,
            payload: Payload::Validation(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Quorum message
    pub fn new_quorum(header: Header, payload: payload::Quorum) -> Message {
        Self {
            header,
            payload: Payload::Quorum(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Block message
    pub fn new_block(payload: Box<ledger::Block>) -> Message {
        Self {
            header: Header::new(Topics::Block),
            payload: Payload::Block(payload),
            ..Default::default()
        }
    }

    /// Creates topics.GetCandidate message
    pub fn new_get_candidate(p: payload::GetCandidate) -> Message {
        Self {
            header: Header::new(Topics::GetCandidate),
            payload: Payload::GetCandidate(p),
            ..Default::default()
        }
    }

    /// Creates topics.Candidate message
    pub fn new_candidate_resp(p: Box<payload::CandidateResp>) -> Message {
        Self {
            header: Header::new(Topics::GetCandidateResp),
            payload: Payload::CandidateResp(p),
            ..Default::default()
        }
    }

    /// Creates topics.Inv (inventory) message
    pub fn new_inv(p: payload::Inv) -> Message {
        Self {
            header: Header::new(Topics::GetInv),
            payload: Payload::GetInv(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetData  message
    pub fn new_get_data(p: payload::Inv) -> Message {
        Self {
            header: Header::new(Topics::GetData),
            payload: Payload::GetInv(p),
            ..Default::default()
        }
    }

    /// Creates topics.GetBlocks  message
    pub fn new_get_blocks(p: payload::GetBlocks) -> Message {
        Self {
            header: Header::new(Topics::GetBlocks),
            payload: Payload::GetBlocks(p),
            ..Default::default()
        }
    }

    /// Creates topics.Tx  message
    pub fn new_transaction(tx: Box<ledger::Transaction>) -> Message {
        Self {
            header: Header::new(Topics::Tx),
            payload: Payload::Transaction(tx),
            ..Default::default()
        }
    }

    /// Creates a message with a validation_result
    pub fn from_validation_result(p: payload::ValidationResult) -> Message {
        Self {
            header: Header::default(),
            payload: Payload::ValidationResult(Box::new(p)),
            ..Default::default()
        }
    }

    /// Creates a unknown message with empty payload
    pub fn empty() -> Message {
        Self {
            header: Header::default(),
            payload: Payload::Empty,
            ..Default::default()
        }
    }

    pub fn topic(&self) -> Topics {
        self.header.topic
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Header {
    pub topic: Topics,

    pub pubkey_bls: bls::PublicKey,
    pub round: u64,
    pub iteration: u8,
    pub block_hash: [u8; 32],
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Header")
            .field("topic", &self.topic)
            .field("pubkey_bls", &to_str(self.pubkey_bls.bytes().inner()))
            .field("round", &self.round)
            .field("iteration", &self.iteration)
            .field("block_hash", &ledger::to_str(&self.block_hash))
            .finish()
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(self.pubkey_bls.bytes().inner())?;
        w.write_all(&self.round.to_le_bytes())?;
        w.write_all(&[self.iteration])?;
        w.write_all(&self.block_hash[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read bls pubkey
        let mut pubkey_bls = [0u8; 96];
        r.read_exact(&mut pubkey_bls)?;
        let pubkey_bls = pubkey_bls
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        // Read round
        let round = Self::read_u64_le(r)?;

        // Read iteration
        let iteration = Self::read_u8(r)?;

        // Read block_hash
        let mut block_hash = [0u8; 32];
        r.read_exact(&mut block_hash)?;

        Ok(Header {
            pubkey_bls,
            round,
            iteration,
            block_hash,
            topic: Topics::default(),
        })
    }
}

impl Header {
    pub fn new(topic: Topics) -> Self {
        Self {
            topic,
            ..Default::default()
        }
    }
    pub fn compare(&self, round: u64, iteration: u8, step: StepName) -> Status {
        self.round
            .cmp(&round)
            .then_with(|| self.get_step().cmp(&step.to_step(iteration)))
            .into()
    }

    pub fn compare_round(&self, round: u64) -> Status {
        if self.round == round {
            return Status::Present;
        }

        if self.round > round {
            return Status::Future;
        }

        Status::Past
    }

    pub fn verify_signature(
        &self,
        signature: &[u8; 48],
    ) -> Result<(), dusk_bls12_381_sign::Error> {
        let sig = dusk_bls12_381_sign::Signature::from_bytes(signature)?;

        dusk_bls12_381_sign::APK::from(self.pubkey_bls.inner()).verify(
            &sig,
            marshal_signable_vote(
                self.round,
                self.get_step(),
                &self.block_hash,
            )
            .bytes(),
        )
    }

    pub fn sign(
        &self,
        sk: &dusk_bls12_381_sign::SecretKey,
        pk: &dusk_bls12_381_sign::PublicKey,
    ) -> [u8; 48] {
        let msg = marshal_signable_vote(
            self.round,
            self.get_step(),
            &self.block_hash,
        );

        sk.sign(pk, msg.bytes()).to_bytes()
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
    CandidateResp(Box<payload::CandidateResp>),

    // Internal messages payload
    /// Result message passed from Validation step to Ratification step
    ValidationResult(Box<payload::ValidationResult>),

    #[default]
    Empty,
}

pub mod payload {
    use crate::ledger::{self, Block, Certificate, StepVotes};
    use crate::Serializable;
    use std::fmt;
    use std::io::{self, Read, Write};

    #[derive(Debug, Clone)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct Ratification {
        pub signature: [u8; 48],
        pub timestamp: u64,
        pub validation_result: ValidationResult,
    }

    #[derive(Debug, Copy, Clone)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct Validation {
        pub signature: [u8; 48],
    }

    impl Serializable for Validation {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.signature)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut signature = [0u8; 48];
            r.read_exact(&mut signature)?;

            Ok(Validation { signature })
        }
    }

    #[derive(Clone)]
    #[cfg_attr(any(feature = "faker", test), derive(fake::Dummy))]
    pub struct Candidate {
        pub signature: [u8; 48],
        pub candidate: Block,
    }

    impl std::fmt::Debug for Candidate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Candidate")
                .field("signature", &ledger::to_str(&self.signature))
                .field("block", &self.candidate)
                .finish()
        }
    }

    impl PartialEq<Self> for Candidate {
        fn eq(&self, other: &Self) -> bool {
            self.signature.eq(&other.signature)
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
            self.candidate.write(w)?;
            w.write_all(&self.signature)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let candidate = Block::read(r)?;
            let mut signature = [0u8; 48];
            r.read_exact(&mut signature)?;

            Ok(Candidate {
                candidate,
                signature,
            })
        }
    }
    #[derive(Clone, Copy, Default)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub enum QuorumType {
        /// Quorum on Valid Candidate
        ValidQuorum = 0,
        // Quorum on Invalid Candidate
        InvalidQuorum = 1,
        //Quorum on Timeout (NilQuorum)
        NilQuorum = 2,
        // NoQuorum
        #[default]
        NoQuorum = 255,
    }

    impl From<u8> for QuorumType {
        fn from(v: u8) -> QuorumType {
            match v {
                0 => QuorumType::ValidQuorum,
                1 => QuorumType::InvalidQuorum,
                2 => QuorumType::NilQuorum,
                _ => QuorumType::NoQuorum,
            }
        }
    }

    impl fmt::Debug for QuorumType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let label = match self {
                QuorumType::ValidQuorum => "valid_quorum",
                QuorumType::InvalidQuorum => "invalid_quorum",
                QuorumType::NilQuorum => "nil_quorum",
                QuorumType::NoQuorum => "no_quorum",
            };
            f.write_str(label)
        }
    }

    #[derive(Debug, Clone, Default)]
    #[cfg_attr(
        any(feature = "faker", test),
        derive(fake::Dummy, Eq, PartialEq)
    )]
    pub struct ValidationResult {
        pub quorum: QuorumType,
        pub hash: [u8; 32],
        pub sv: StepVotes,
    }

    #[derive(Debug, Clone, Eq, Hash, PartialEq)]
    pub struct Quorum {
        pub signature: [u8; 48],
        pub validation: StepVotes,
        pub ratification: StepVotes,
    }

    impl Serializable for Quorum {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.signature)?;
            self.validation.write(w)?;
            self.ratification.write(w)?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut signature = [0u8; 48];
            r.read_exact(&mut signature)?;

            let validation = StepVotes::read(r)?;
            let ratification = StepVotes::read(r)?;

            Ok(Quorum {
                signature,
                validation,
                ratification,
            })
        }
    }

    impl Quorum {
        /// Generates a certificate from quorum.
        pub fn generate_certificate(&self) -> Certificate {
            Certificate {
                validation: self.validation,
                ratification: self.ratification,
            }
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
            let mut result = GetCandidate::default();
            r.read_exact(&mut result.hash[..])?;

            Ok(result)
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct CandidateResp {
        pub candidate: Block,
    }

    impl Serializable for CandidateResp {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.candidate.write(w)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(CandidateResp {
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
                        let mut hash = [0u8; 32];
                        r.read_exact(&mut hash)?;

                        inv.add_tx_hash(hash);
                    }
                    InvType::BlockFromHash => {
                        let mut hash = [0u8; 32];
                        r.read_exact(&mut hash)?;

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
            let mut locator = [0u8; 32];
            r.read_exact(&mut locator)?;

            Ok(Self { locator })
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct GetData {
        pub inner: Inv,
    }

    impl Serializable for GetData {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.inner.write(w)
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(GetData {
                inner: Inv::read(r)?,
            })
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

impl<M: Clone> Default for AsyncQueue<M> {
    fn default() -> Self {
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

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use crate::ledger;
    use crate::ledger::*;
    use crate::Serializable;

    #[test]
    fn test_serialize() {
        assert_serialize(crate::message::Header {
            pubkey_bls: bls::PublicKey::from_sk_seed_u64(1),
            round: 8,
            iteration: 7,
            block_hash: [3; 32],
            topic: Topics::Unknown,
        });

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
            },
            iteration: 1,
            prev_block_cert: Certificate {
                validation: ledger::StepVotes::new([6; 48], 444444444),
                ratification: ledger::StepVotes::new([7; 48], 55555555),
            },
            failed_iterations: Default::default(),
        };

        let sample_block =
            ledger::Block::new(header, vec![]).expect("should be valid block");

        assert_serialize(payload::Candidate {
            candidate: sample_block,
            signature: [4; 48],
        });

        assert_serialize(ledger::StepVotes {
            bitset: 12345,
            aggregate_signature: Signature([4; 48]),
        });

        assert_serialize(payload::Validation { signature: [4; 48] });

        assert_serialize(ledger::StepVotes {
            bitset: 12345,
            aggregate_signature: Signature([4; 48]),
        });

        assert_serialize(payload::Quorum {
            validation: ledger::StepVotes {
                bitset: 12345,
                aggregate_signature: Signature([1; 48]),
            },
            ratification: ledger::StepVotes {
                bitset: 98765,
                aggregate_signature: Signature([2; 48]),
            },
            signature: [3; 48],
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
