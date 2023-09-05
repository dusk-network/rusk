// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::{Buf, BufMut, BytesMut};
use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable as DuskSerializable;

use crate::ledger::to_str;
use crate::{bls, ledger, Serializable};
use std::io::{self, Read, Write};
use std::net::SocketAddr;

use async_channel::TrySendError;

/// Topic field position in the message binary representation
pub const TOPIC_FIELD_POS: usize = 8 + 8 + 8 + 4;

pub enum Status {
    Past,
    Present,
    Future,
}

pub fn marshal_signable_vote(
    round: u64,
    step: u8,
    block_hash: &[u8; 32],
) -> BytesMut {
    let mut msg = BytesMut::with_capacity(block_hash.len() + 8 + 1);
    msg.put_u64_le(round);
    msg.put_u8(step);
    msg.put(&block_hash[..]);

    msg
}

pub trait MessageTrait {
    fn compare(&self, round: u64, step: u8) -> Status;
    fn get_pubkey_bls(&self) -> &bls::PublicKey;
    fn get_block_hash(&self) -> [u8; 32];
    fn get_topic(&self) -> u8;
    fn get_step(&self) -> u8;
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
        w.write_all(&[self.header.topic])?;

        // Optional header fields used only for consensus messages
        if is_consensus_msg(self.header.topic) {
            self.header.write(w)?;
        }

        match &self.payload {
            Payload::NewBlock(p) => p.write(w),
            Payload::Reduction(p) => p.write(w),
            Payload::Agreement(p) => p.write(w),
            Payload::AggrAgreement(p) => p.write(w),
            Payload::Block(p) => p.write(w),
            Payload::Transaction(p) => p.write(w),
            Payload::GetCandidate(p) => p.write(w),
            Payload::CandidateResp(p) => p.write(w),
            Payload::GetMempool(p) => p.write(w),
            Payload::GetInv(p) => p.write(w),
            Payload::GetBlocks(p) => p.write(w),
            Payload::GetData(p) => p.write(w),
            Payload::Empty
            | Payload::StepVotes(_)
            | Payload::StepVotesWithCandidate(_) => Ok(()), /* interal message, not sent on the wire */
        }
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read topic
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        let topic = Topics::from(buf[0]);
        if topic == Topics::Unknown {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown topic",
            ));
        }

        // Decode message header only if the topic is supported
        let mut header = Header::default();
        if is_consensus_msg(buf[0]) {
            header = Header::read(r)?;
        }

        header.topic = buf[0];

        let payload = match topic {
            Topics::NewBlock => {
                Payload::NewBlock(Box::new(payload::NewBlock::read(r)?))
            }
            Topics::FirstReduction | Topics::SecondReduction => {
                Payload::Reduction(payload::Reduction::read(r)?)
            }
            Topics::Agreement => {
                Payload::Agreement(payload::Agreement::read(r)?)
            }
            Topics::AggrAgreement => {
                Payload::AggrAgreement(payload::AggrAgreement::read(r)?)
            }
            Topics::Block => Payload::Block(Box::new(ledger::Block::read(r)?)),
            Topics::Tx => {
                Payload::Transaction(Box::new(ledger::Transaction::read(r)?))
            }
            Topics::Candidate => Payload::CandidateResp(Box::new(
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
    fn compare(&self, round: u64, step: u8) -> Status {
        self.header.compare(round, step)
    }
    fn get_pubkey_bls(&self) -> &bls::PublicKey {
        &self.header.pubkey_bls
    }
    fn get_block_hash(&self) -> [u8; 32] {
        self.header.block_hash
    }
    fn get_topic(&self) -> u8 {
        self.header.topic
    }

    fn get_step(&self) -> u8 {
        self.header.step
    }
}

impl Message {
    /// Creates topics.NewBlock message
    pub fn new_newblock(header: Header, p: payload::NewBlock) -> Message {
        Self {
            header,
            payload: Payload::NewBlock(Box::new(p)),
            ..Default::default()
        }
    }

    /// Creates topics.Reduction message
    pub fn new_reduction(
        header: Header,
        payload: payload::Reduction,
    ) -> Message {
        Self {
            header,
            payload: Payload::Reduction(payload),
            ..Default::default()
        }
    }

    /// Creates topics.Agreement message
    pub fn new_agreement(
        header: Header,
        payload: payload::Agreement,
    ) -> Message {
        Self {
            header,
            payload: Payload::Agreement(payload),
            ..Default::default()
        }
    }

    /// Creates topics.AggrAgreement message
    pub fn new_aggr_agreement(
        header: Header,
        payload: payload::AggrAgreement,
    ) -> Message {
        Self {
            header,
            payload: Payload::AggrAgreement(payload),
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
            header: Header::new(Topics::Candidate),
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

    /// Creates a message with a step votes payload
    /// This is never sent on the wire, but used internally
    pub fn from_stepvotes(p: payload::StepVotesWithCandidate) -> Message {
        Self {
            header: Header::default(),
            payload: Payload::StepVotesWithCandidate(Box::new(p)),
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
        Topics::from(self.header.topic)
    }
}

fn is_consensus_msg(topic: u8) -> bool {
    matches!(
        Topics::from(topic),
        Topics::NewBlock
            | Topics::FirstReduction
            | Topics::SecondReduction
            | Topics::Agreement
            | Topics::AggrAgreement
    )
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Header {
    pub topic: u8,

    pub pubkey_bls: bls::PublicKey,
    pub round: u64,
    pub step: u8,
    pub block_hash: [u8; 32],
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Header")
            .field("topic", &self.topic)
            .field("pubkey_bls", &to_str(self.pubkey_bls.bytes()))
            .field("round", &self.round)
            .field("step", &self.step)
            .field("block_hash", &ledger::to_str(&self.block_hash))
            .finish()
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        Self::write_var_bytes(w, &self.pubkey_bls.bytes()[..])?;
        w.write_all(&self.round.to_le_bytes())?;
        w.write_all(&[self.step])?;
        w.write_all(&self.block_hash[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read bls pubkey
        let buf: [u8; 96] = Self::read_var_bytes(r)?
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut pubkey_bls = bls::PublicKey::default();
        if buf != [0u8; 96] {
            pubkey_bls = match dusk_bls12_381_sign::PublicKey::from_slice(&buf)
            {
                Ok(pk) => bls::PublicKey::new(pk),
                Err(_) => {
                    return Ok(Header::default()); // TODO: This should be an
                                                  // error
                }
            }
        }

        // Read round
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let round = u64::from_le_bytes(buf);

        // Read step
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let step = buf[0];

        // Read block_hash
        let mut block_hash = [0u8; 32];
        r.read_exact(&mut block_hash[..])?;

        Ok(Header {
            pubkey_bls,
            round,
            step,
            block_hash,
            topic: 0,
        })
    }
}

impl Header {
    pub fn new(topic: Topics) -> Self {
        Self {
            topic: topic as u8,
            ..Default::default()
        }
    }

    pub fn compare(&self, round: u64, step: u8) -> Status {
        if self.round == round {
            if self.step == step {
                return Status::Present;
            }

            if self.step > step {
                return Status::Future;
            }

            return Status::Past;
        }

        if self.round > round {
            return Status::Future;
        }

        Status::Past
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
            marshal_signable_vote(self.round, self.step, &self.block_hash)
                .bytes(),
        )
    }

    pub fn sign(
        &self,
        sk: &dusk_bls12_381_sign::SecretKey,
        pk: &dusk_bls12_381_sign::PublicKey,
    ) -> [u8; 48] {
        let mut msg = BytesMut::with_capacity(self.block_hash.len() + 8 + 1);
        msg.put_u64_le(self.round);
        msg.put_u8(self.step);
        msg.put(&self.block_hash[..]);

        sk.sign(pk, msg.bytes()).to_bytes()
    }
}

#[derive(Default, Debug, Clone)]
pub enum Payload {
    Reduction(payload::Reduction),
    NewBlock(Box<payload::NewBlock>),
    Agreement(payload::Agreement),
    AggrAgreement(payload::AggrAgreement),

    StepVotes(ledger::StepVotes),
    StepVotesWithCandidate(Box<payload::StepVotesWithCandidate>),

    Block(Box<ledger::Block>),
    Transaction(Box<ledger::Transaction>),
    GetCandidate(payload::GetCandidate),
    GetMempool(payload::GetMempool),
    GetInv(payload::Inv),
    GetBlocks(payload::GetBlocks),
    GetData(payload::GetData),
    CandidateResp(Box<payload::CandidateResp>),

    #[default]
    Empty,
}

pub mod payload {
    use crate::ledger::{self, Block, Certificate, StepVotes};
    use crate::Serializable;
    use std::io::{self, Read, Write};

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct Reduction {
        pub signature: [u8; 48],
    }

    impl Serializable for Reduction {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            Self::write_var_bytes(w, &self.signature[..])?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let signature: [u8; 48] = Self::read_var_bytes(r)?
                .try_into()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

            Ok(Reduction { signature })
        }
    }

    impl Default for Reduction {
        fn default() -> Self {
            Self { signature: [0; 48] }
        }
    }

    #[derive(Clone)]
    pub struct NewBlock {
        pub signature: [u8; 48],
        pub candidate: Block,
    }

    impl std::fmt::Debug for NewBlock {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NewBlock")
                .field("signature", &ledger::to_str(&self.signature))
                .field("candidate", &self.candidate)
                .finish()
        }
    }

    impl Default for NewBlock {
        fn default() -> Self {
            Self {
                candidate: Default::default(),
                signature: [0; 48],
            }
        }
    }

    impl PartialEq<Self> for NewBlock {
        fn eq(&self, other: &Self) -> bool {
            self.signature.eq(&other.signature)
                && self
                    .candidate
                    .header()
                    .hash
                    .eq(&other.candidate.header().hash)
        }
    }

    impl Eq for NewBlock {}

    impl Serializable for NewBlock {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.candidate.write(w)?;
            Self::write_var_bytes(w, &self.signature[..])?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            Ok(NewBlock {
                candidate: Block::read(r)?,
                signature: Self::read_var_bytes(r)?
                    .try_into()
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?,
            })
        }
    }

    #[derive(Debug, Clone)]
    pub struct StepVotesWithCandidate {
        pub sv: StepVotes,
        pub candidate: Block,
    }

    #[derive(Debug, Clone, Eq, Hash, PartialEq)]
    pub struct Agreement {
        pub signature: [u8; 48],

        /// StepVotes of both 1th and 2nd Reduction steps
        pub first_step: StepVotes,
        pub second_step: StepVotes,
    }

    impl Serializable for Agreement {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            Self::write_var_bytes(w, &self.signature[..])?;

            // Read this field for backward compatibility
            let step_votes_len = 2u8;
            w.write_all(&step_votes_len.to_le_bytes())?;

            self.first_step.write(w)?;
            self.second_step.write(w)?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let signature = Self::read_var_bytes(r)?
                .try_into()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

            let mut step_votes_len = [0u8; 1];
            r.read_exact(&mut step_votes_len)?;

            let first_step = StepVotes::read(r)?;
            let second_step = StepVotes::read(r)?;

            Ok(Agreement {
                signature,
                first_step,
                second_step,
            })
        }
    }

    impl Default for Agreement {
        fn default() -> Self {
            Self {
                signature: [0; 48],
                first_step: StepVotes::default(),
                second_step: StepVotes::default(),
            }
        }
    }

    impl Agreement {
        /// Generates a certificate from agreement.
        pub fn generate_certificate(&self) -> Certificate {
            Certificate {
                first_reduction: self.first_step,
                second_reduction: self.second_step,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AggrAgreement {
        pub agreement: Agreement,
        pub bitset: u64,
        pub aggregate_signature: [u8; 48],
    }

    impl Serializable for AggrAgreement {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.agreement.write(w)?;
            w.write_all(&self.bitset.to_le_bytes())?;
            Self::write_var_bytes(w, &self.aggregate_signature[..])?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let agreement = Agreement::read(r)?;

            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;
            let bitset = u64::from_le_bytes(buf);

            let aggr_signature = Self::read_var_bytes(r)?
                .try_into()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

            Ok(AggrAgreement {
                agreement,
                bitset,
                aggregate_signature: aggr_signature,
            })
        }
    }

    impl Default for AggrAgreement {
        fn default() -> Self {
            Self {
                aggregate_signature: [0; 48],
                agreement: Default::default(),
                bitset: 0,
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
        Block,
    }

    #[derive(Default, Debug, Clone, Copy)]
    pub struct InvVect {
        pub inv_type: InvType,
        pub hash: [u8; 32],
    }

    #[derive(Default, Debug, Clone)]
    pub struct Inv {
        pub inv_list: Vec<InvVect>,
    }

    impl Inv {
        pub fn add_tx_hash(&mut self, hash: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::MempoolTx,
                hash,
            });
        }

        pub fn add_block_hash(&mut self, hash: [u8; 32]) {
            self.inv_list.push(InvVect {
                inv_type: InvType::Block,
                hash,
            });
        }
    }

    impl Serializable for Inv {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            Self::write_varint(w, self.inv_list.len() as u64)?;

            for item in &self.inv_list {
                w.write_all(&[item.inv_type as u8])?;
                w.write_all(&item.hash[..])?;
            }

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let items_len = Self::read_varint(r)?;

            let mut inv = Inv::default();
            for _ in 0..items_len {
                let mut inv_type_buf = [0u8; 1];
                r.read_exact(&mut inv_type_buf)?;

                let inv_type = match inv_type_buf[0] {
                    0 => InvType::MempoolTx,
                    1 => InvType::Block,
                    _ => {
                        return Err(io::Error::from(io::ErrorKind::InvalidData))
                    }
                };

                let mut hash = [0u8; 32];
                r.read_exact(&mut hash)?;

                inv.inv_list.push(InvVect { inv_type, hash });
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
            Self::write_varint(w, 1)?;
            w.write_all(&self.locator[..])
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut result = GetBlocks::default();
            Self::read_varint(r)?;
            r.read_exact(&mut result.locator[..])?;

            Ok(result)
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
    Candidate = 15,
    NewBlock = 16,
    FirstReduction = 17,
    SecondReduction = 18,

    // Consensus Agreement loop topics
    Agreement = 19,
    AggrAgreement = 20,

    #[default]
    Unknown = 255,
}

impl From<u8> for Topics {
    fn from(v: u8) -> Self {
        map_topic!(v, Topics::GetData);
        map_topic!(v, Topics::GetBlocks);
        map_topic!(v, Topics::Tx);
        map_topic!(v, Topics::Block);
        map_topic!(v, Topics::GetMempool);
        map_topic!(v, Topics::GetInv);
        map_topic!(v, Topics::Candidate);
        map_topic!(v, Topics::GetCandidate);
        map_topic!(v, Topics::NewBlock);
        map_topic!(v, Topics::FirstReduction);
        map_topic!(v, Topics::SecondReduction);
        map_topic!(v, Topics::Agreement);
        map_topic!(v, Topics::AggrAgreement);

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
            step: 7,
            block_hash: [3; 32],
            topic: 0,
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
                first_reduction: ledger::StepVotes::new([6; 48], 22222222),
                second_reduction: ledger::StepVotes::new([7; 48], 3333333),
            },
            iteration: 1,
        };

        let sample_block =
            ledger::Block::new(header, vec![]).expect("should be valid block");

        assert_serialize(payload::NewBlock {
            candidate: sample_block,
            signature: [4; 48],
        });

        assert_serialize(payload::AggrAgreement {
            agreement: payload::Agreement {
                first_step: StepVotes {
                    bitset: 12345,
                    aggregate_signature: Signature([1; 48]),
                },
                second_step: StepVotes {
                    bitset: 98765,
                    aggregate_signature: Signature([2; 48]),
                },
                signature: [3; 48],
            },
            aggregate_signature: [8; 48],
            bitset: 10,
        });

        assert_serialize(ledger::StepVotes {
            bitset: 12345,
            aggregate_signature: Signature([4; 48]),
        });

        assert_serialize(payload::Reduction { signature: [4; 48] });

        assert_serialize(ledger::StepVotes {
            bitset: 12345,
            aggregate_signature: Signature([4; 48]),
        });

        assert_serialize(payload::Agreement {
            first_step: ledger::StepVotes {
                bitset: 12345,
                aggregate_signature: Signature([1; 48]),
            },
            second_step: ledger::StepVotes {
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
