// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{commons::Topics, util::pubkey::ConsensusPublicKey};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use dusk_bytes::DeserializableSlice;

pub enum Status {
    Past,
    Present,
    Future,
}
///
pub trait Serializable {
    /// Serialize struct to Vec<u8>.
    fn to_bytes(&self) -> Vec<u8>;

    /// Deserialize struct from buf by consuming N bytes.
    fn from_bytes(buf: &mut Bytes) -> Self;
}
pub trait MessageTrait {
    fn compare(&self, round: u64, step: u8) -> Status;
    fn get_pubkey_bls(&self) -> ConsensusPublicKey;
    fn get_block_hash(&self) -> [u8; 32];
}

/// Message is a data unit that consensus phase can process.
#[derive(Debug, Default, Clone)]
pub struct Message {
    pub header: Header,
    pub payload: Payload,

    pub metadata: Option<TransportData>,
}

/// Defines a transport-related properties that determines how the message
/// will be broadcast.
/// TODO: This should be moved out of consensus message definition.
#[derive(Debug, Clone)]
pub struct TransportData {
    pub height: u8,
    pub src_addr: String,
}

impl Serializable for Message {
    /// Support serialization for messages that are sent on the wire.
    fn to_bytes(&self) -> Vec<u8> {
        let payload_as_vec = match &self.payload {
            Payload::NewBlock(p) => p.to_bytes(),
            Payload::Reduction(p) => p.to_bytes(),
            Payload::Agreement(p) => p.to_bytes(),
            _ => vec![], // non-serialziable messages are those which are not sent on the wire.
        };

        let mut buf = BytesMut::with_capacity(payload_as_vec.len());
        buf.put(&self.header.to_bytes()[..]);
        buf.put(&payload_as_vec[..]);
        buf.to_vec()
    }

    // Support de-serialization  for messages that are received from the wire.
    fn from_bytes(buf: &mut Bytes) -> Self {
        let mut msg = Self {
            header: Header::from_bytes(buf),
            payload: Payload::Empty,
            metadata: Default::default(),
        };

        msg.payload = match Topics::from(msg.header.topic) {
            Topics::NewBlock => Payload::NewBlock(Box::new(payload::NewBlock::from_bytes(buf))),
            Topics::Reduction => Payload::Reduction(payload::Reduction::from_bytes(buf)),
            Topics::Agreement => Payload::Agreement(payload::Agreement::from_bytes(buf)),
            _ => {
                debug_assert!(false, "unhandled topic {}", msg.header.topic);
                Payload::Empty
            }
        };

        msg
    }
}

impl MessageTrait for Message {
    fn compare(&self, round: u64, step: u8) -> Status {
        self.header.compare(round, step)
    }
    fn get_pubkey_bls(&self) -> ConsensusPublicKey {
        self.header.pubkey_bls
    }
    fn get_block_hash(&self) -> [u8; 32] {
        self.header.block_hash
    }
}

impl Message {
    pub fn from_newblock(header: Header, p: payload::NewBlock) -> Message {
        Self {
            header,
            payload: Payload::NewBlock(Box::new(p)),
            ..Default::default()
        }
    }

    pub fn from_stepvotes(p: payload::StepVotesWithCandidate) -> Message {
        Self {
            header: Header::default(),
            payload: Payload::StepVotesWithCandidate(p),
            ..Default::default()
        }
    }

    pub fn new_reduction(header: Header, payload: payload::Reduction) -> Message {
        Self {
            header,
            payload: Payload::Reduction(payload),
            ..Default::default()
        }
    }

    pub fn new_agreement(header: Header, payload: payload::Agreement) -> Message {
        Self {
            header,
            payload: Payload::Agreement(payload),
            ..Default::default()
        }
    }

    pub fn empty() -> Message {
        Self {
            header: Header::default(),
            payload: Payload::Empty,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub pubkey_bls: ConsensusPublicKey,
    pub round: u64,
    pub step: u8,
    pub block_hash: [u8; 32],

    pub topic: u8,
}

impl Serializable for Header {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(300);
        buf.put_u8(self.topic);
        buf.put_u8(self.step);
        buf.put_u64_le(self.round);
        buf.put(&self.block_hash[..]);
        buf.put(&self.pubkey_bls.bytes()[..]);

        buf.to_vec()
    }

    fn from_bytes(buf: &mut Bytes) -> Self {
        let mut header = Header {
            topic: buf.get_u8(),
            step: buf.get_u8(),
            round: buf.get_u64_le(),
            ..Default::default()
        };
        buf.copy_to_slice(&mut header.block_hash[..]);

        let mut pubkey_bytes = [0u8; 96];
        buf.copy_to_slice(&mut pubkey_bytes[..]);

        header.pubkey_bls = ConsensusPublicKey::new(
            dusk_bls12_381_sign::PublicKey::from_slice(&pubkey_bytes).unwrap(),
        );

        header
    }
}

impl Header {
    pub fn compare(&self, round: u64, step: u8) -> Status {
        if self.round == round {
            if self.step == step {
                return Status::Present;
            }

            if self.step > step {
                return Status::Future;
            }

            if self.step < step {
                return Status::Past;
            }
        }

        if self.round > round {
            return Status::Future;
        }

        if self.round < round {
            return Status::Past;
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
}

#[derive(Debug, Clone)]
pub enum Payload {
    Reduction(payload::Reduction),
    NewBlock(Box<payload::NewBlock>),
    StepVotes(payload::StepVotes),
    StepVotesWithCandidate(payload::StepVotesWithCandidate),
    Agreement(payload::Agreement),
    Empty,
}

impl Default for Payload {
    fn default() -> Self {
        Payload::Empty
    }
}

pub mod payload {
    use super::Serializable;
    use crate::commons::Block;
    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use std::mem;

    #[derive(Debug, Copy, Clone)]
    pub struct Reduction {
        pub signed_hash: [u8; 48],
    }

    impl Serializable for Reduction {
        fn to_bytes(&self) -> Vec<u8> {
            let mut buf = BytesMut::with_capacity(48);
            buf.put(&self.signed_hash[..]);
            buf.to_vec()
        }

        fn from_bytes(buf: &mut Bytes) -> Self {
            let mut r = Self {
                signed_hash: [0; 48],
            };

            buf.copy_to_slice(&mut r.signed_hash);
            r
        }
    }

    impl Default for Reduction {
        fn default() -> Self {
            Self {
                signed_hash: [0; 48],
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct NewBlock {
        pub prev_hash: [u8; 32],
        pub candidate: Block,
        pub signed_hash: [u8; 48],
    }

    impl Default for NewBlock {
        fn default() -> Self {
            Self {
                candidate: Default::default(),
                prev_hash: Default::default(),
                signed_hash: [0; 48],
            }
        }
    }

    impl Serializable for NewBlock {
        fn to_bytes(&self) -> Vec<u8> {
            let candidate_as_bytes = self.candidate.to_bytes();

            let mut buf = BytesMut::with_capacity(candidate_as_bytes.len() + 80);
            buf.put(&self.prev_hash[..]);
            buf.put(&self.signed_hash[..]);
            buf.put(&candidate_as_bytes[..]);

            buf.to_vec()
        }

        fn from_bytes(buf: &mut Bytes) -> Self {
            let mut nb = NewBlock::default();

            buf.copy_to_slice(&mut nb.prev_hash);
            buf.copy_to_slice(&mut nb.signed_hash);

            nb.candidate.from_bytes(buf);
            nb
        }
    }

    #[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
    pub struct StepVotes {
        pub bitset: u64,
        pub signature: [u8; 48],
    }

    impl Default for StepVotes {
        fn default() -> Self {
            Self {
                bitset: 0,
                signature: [0; 48],
            }
        }
    }

    impl StepVotes {
        pub fn to_bytes(&self) -> Vec<u8> {
            let mut buf = BytesMut::with_capacity(mem::size_of::<StepVotes>());
            buf.put_u64(self.bitset);
            buf.put(&self.signature[..]);
            buf.to_vec()
        }

        pub fn from_bytes(&mut self, buf: &mut Bytes) {
            self.bitset = buf.get_u64();
            buf.copy_to_slice(&mut self.signature);
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
        fn to_bytes(&self) -> Vec<u8> {
            let mut buf = BytesMut::with_capacity(48);
            buf.put(&self.signature[..]);
            buf.put(&self.first_step.to_bytes()[..]);
            buf.put(&self.second_step.to_bytes()[..]);
            buf.to_vec()
        }

        fn from_bytes(buf: &mut Bytes) -> Self {
            let mut agr = Agreement::default();

            buf.copy_to_slice(&mut agr.signature);

            agr.first_step.from_bytes(buf);
            agr.second_step.from_bytes(buf);
            agr
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
}
