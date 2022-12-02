// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::{Buf, BufMut, BytesMut};
use dusk_bytes::DeserializableSlice;
use dusk_bytes::Serializable as DuskSerializable;

use crate::commons::{marshal_signable_vote, Topics};
use crate::util::pubkey::ConsensusPublicKey;
use std::io::{self, Read, Write};

pub enum Status {
    Past,
    Present,
    Future,
}

pub trait Serializable {
    /// Serialize struct to Vec<u8>.
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    fn read<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;

    fn write_var_le_bytes<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u8;

        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;

        Ok(())
    }

    fn read_var_le_bytes<R: Read, const N: usize>(
        r: &mut R,
    ) -> io::Result<[u8; N]> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        let mut buf = [0u8; N];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
}

pub trait MessageTrait {
    fn compare(&self, round: u64, step: u8) -> Status;
    fn get_pubkey_bls(&self) -> &ConsensusPublicKey;
    fn get_block_hash(&self) -> [u8; 32];
}

/// Message is a data unit that consensus phase can process.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Message {
    pub header: Header,
    pub payload: Payload,

    pub metadata: Option<TransportData>,
}

/// Defines a transport-related properties that determines how the message
/// will be broadcast.
/// TODO: This should be moved out of consensus message definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportData {
    pub height: u8,
    pub src_addr: String,
}

impl Serializable for Message {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;

        match &self.payload {
            Payload::NewBlock(p) => p.write(w),
            Payload::Reduction(p) => p.write(w),
            Payload::Agreement(p) => p.write(w),
            Payload::AggrAgreement(p) => p.write(w),
            _ => Ok(()), // non-serialziable messages are those which are not sent on the wire.
        }
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = Header::read(r)?;
        let payload = match Topics::from(header.topic) {
            Topics::NewBlock => {
                Payload::NewBlock(Box::new(payload::NewBlock::read(r)?))
            }
            Topics::Reduction => {
                Payload::Reduction(payload::Reduction::read(r)?)
            }
            Topics::Agreement => {
                Payload::Agreement(payload::Agreement::read(r)?)
            }
            Topics::AggrAgreement => {
                Payload::AggrAgreement(payload::AggrAgreement::read(r)?)
            }
            _ => {
                debug_assert!(false, "unhandled topic {}", header.topic);
                Payload::Empty
            }
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
    fn get_pubkey_bls(&self) -> &ConsensusPublicKey {
        &self.header.pubkey_bls
    }
    fn get_block_hash(&self) -> [u8; 32] {
        self.header.block_hash
    }
}

impl Message {
    pub fn new_newblock(header: Header, p: payload::NewBlock) -> Message {
        Self {
            header,
            payload: Payload::NewBlock(Box::new(p)),
            ..Default::default()
        }
    }

    pub fn from_stepvotes(p: payload::StepVotesWithCandidate) -> Message {
        Self {
            header: Header::default(),
            payload: Payload::StepVotesWithCandidate(Box::new(p)),
            ..Default::default()
        }
    }

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

    pub fn empty() -> Message {
        Self {
            header: Header::default(),
            payload: Payload::Empty,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Header {
    pub pubkey_bls: ConsensusPublicKey,
    pub round: u64,
    pub step: u8,
    pub block_hash: [u8; 32],

    pub topic: u8,
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&[self.topic])?;
        Self::write_var_le_bytes(w, &self.pubkey_bls.bytes()[..])?;
        w.write_all(&self.round.to_le_bytes())?;
        w.write_all(&[self.step])?;
        w.write_all(&self.block_hash[..])?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read topic
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let topic = buf[0];

        // Read bls pubkey
        let buf: [u8; 96] = Self::read_var_le_bytes(r)?;

        let mut pubkey_bls = ConsensusPublicKey::default();
        if buf != [0u8; 96] {
            pubkey_bls = match dusk_bls12_381_sign::PublicKey::from_slice(&buf)
            {
                Ok(pk) => ConsensusPublicKey::new(pk),
                Err(_) => {
                    return Ok(Header::default()); // TODO: This should be an error
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
            topic,
        })
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    Reduction(payload::Reduction),
    NewBlock(Box<payload::NewBlock>),
    StepVotes(payload::StepVotes),
    StepVotesWithCandidate(Box<payload::StepVotesWithCandidate>),
    Agreement(payload::Agreement),
    AggrAgreement(payload::AggrAgreement),
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
    use std::io::{self, Read, Write};

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct Reduction {
        pub signed_hash: [u8; 48],
    }

    impl Serializable for Reduction {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            Self::write_var_le_bytes(w, &self.signed_hash[..])?;
            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let signed_hash = Self::read_var_le_bytes(r)?;
            Ok(Reduction { signed_hash })
        }
    }

    impl Default for Reduction {
        fn default() -> Self {
            Self {
                signed_hash: [0; 48],
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
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
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.prev_hash[..])?;
            self.candidate.write(w)?;
            Self::write_var_le_bytes(w, &self.signed_hash[..])?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut result = NewBlock::default();

            r.read_exact(&mut result.prev_hash[..])?;
            result.candidate = Block::read(r)?;
            result.signed_hash = Self::read_var_le_bytes(r)?;

            Ok(result)
        }
    }

    #[derive(Debug, Clone, Eq, Hash, PartialEq)]
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

    impl Serializable for StepVotes {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            w.write_all(&self.bitset.to_le_bytes())?;
            Self::write_var_le_bytes(w, &self.signature[..])?;

            Ok(())
        }

        fn read<R: Read>(r: &mut R) -> io::Result<Self>
        where
            Self: Sized,
        {
            let mut buf = [0u8; 8];
            r.read_exact(&mut buf[..])?;
            let signature = Self::read_var_le_bytes(r)?;

            Ok(StepVotes {
                bitset: u64::from_le_bytes(buf),
                signature,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
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
            Self::write_var_le_bytes(w, &self.signature[..])?;

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
            let signature = Self::read_var_le_bytes(r)?;

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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct AggrAgreement {
        pub agreement: Agreement,
        pub bitset: u64,
        pub aggr_signature: [u8; 48],
    }

    impl Serializable for AggrAgreement {
        fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
            self.agreement.write(w)?;
            w.write_all(&self.bitset.to_le_bytes())?;
            Self::write_var_le_bytes(w, &self.aggr_signature[..])?;

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

            let aggr_signature = Self::read_var_le_bytes(r)?;

            Ok(AggrAgreement {
                agreement,
                bitset,
                aggr_signature,
            })
        }
    }

    impl Default for AggrAgreement {
        fn default() -> Self {
            Self {
                aggr_signature: [0; 48],
                agreement: Default::default(),
                bitset: 0,
            }
        }
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use crate::commons::{Block, Certificate, Seed, Topics};
    use crate::messages::payload::{Agreement, NewBlock, Reduction, StepVotes};
    use crate::messages::{self, Header, Message, Serializable};
    use crate::util::pubkey::ConsensusPublicKey;

    use super::payload::AggrAgreement;

    #[test]
    fn test_serialize() {
        assert_serialize(Header {
            pubkey_bls: ConsensusPublicKey::from_sk_seed_u64(1),
            round: 8,
            step: 7,
            block_hash: [3; 32],
            topic: 3,
        });

        let sample_block = Block {
            header: crate::commons::Header {
                version: 3,
                height: 1888881,
                timestamp: 123456789,
                gas_limit: 111111111,
                prev_block_hash: [1; 32],
                seed: Seed::new([2; 48]),
                generator_bls_pubkey: [3; 96],
                state_hash: [4; 32],
                hash: [5; 32],
                cert: Certificate {
                    first_reduction: ([6; 48], 22222222),
                    second_reduction: ([7; 48], 3333333),
                    step: 234,
                },
            },
            txs: vec![],
        };

        assert_serialize(NewBlock {
            prev_hash: [3; 32],
            candidate: sample_block,
            signed_hash: [4; 48],
        });

        assert_serialize(StepVotes {
            bitset: 12345,
            signature: [4; 48],
        });

        assert_serialize(Agreement {
            first_step: StepVotes {
                bitset: 12345,
                signature: [1; 48],
            },
            second_step: StepVotes {
                bitset: 98765,
                signature: [2; 48],
            },
            signature: [3; 48],
        });

        assert_serialize(AggrAgreement {
            agreement: Agreement {
                first_step: StepVotes {
                    bitset: 12345,
                    signature: [1; 48],
                },
                second_step: StepVotes {
                    bitset: 98765,
                    signature: [2; 48],
                },
                signature: [3; 48],
            },
            aggr_signature: [8; 48],
            bitset: 10,
        });

        assert_serialize(Reduction {
            signed_hash: [4; 48],
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
