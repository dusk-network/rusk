// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::contract_state::Operations;
// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.
use crate::messages::{self, Message, Serializable};

use crate::util::pending_queue::PendingQueue;
use crate::util::pubkey::ConsensusPublicKey;
use bytes::{BufMut, BytesMut};
use dusk_bls12_381_sign::SecretKey;
use sha3::Digest;
use std::io::{self, Read, Write};

use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type Seed = Signature;
pub type Hash = [u8; 32];

#[derive(Clone, Default, Debug)]
#[allow(unused)]
pub struct RoundUpdate {
    pub round: u64,
    pub seed: Seed,
    pub hash: [u8; 32],
    pub timestamp: i64,
    pub pubkey_bls: ConsensusPublicKey,
    pub secret_key: SecretKey, // TODO: should be here?? SecretKey
}

impl RoundUpdate {
    pub fn new(
        round: u64,
        pubkey_bls: ConsensusPublicKey,
        secret_key: SecretKey,
        seed: Seed,
    ) -> Self {
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            seed,
            hash: [0u8; 32],
            timestamp: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    pub first_reduction: ([u8; 48], u64),
    pub second_reduction: ([u8; 48], u64),
    pub step: u8,
}

impl Default for Certificate {
    fn default() -> Self {
        Self {
            first_reduction: ([0u8; 48], 0),
            second_reduction: ([0u8; 48], 0),
            step: 0,
        }
    }
}

impl Serializable for Certificate {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        Self::write_var_le_bytes(w, &self.first_reduction.0[..])?;
        Self::write_var_le_bytes(w, &self.second_reduction.0[..])?;
        w.write_all(&self.step.to_le_bytes())?;
        w.write_all(&self.first_reduction.1.to_le_bytes())?;
        w.write_all(&self.second_reduction.1.to_le_bytes())?;

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut first_reduction = (Self::read_var_le_bytes(r)?, 0u64);
        let mut second_reduction = (Self::read_var_le_bytes(r)?, 0u64);

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let step = buf[0];

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        first_reduction.1 = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        second_reduction.1 = u64::from_le_bytes(buf);

        Ok(Certificate {
            first_reduction,
            second_reduction,
            step,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    // Hashable fields
    pub version: u8,
    pub height: u64,
    pub timestamp: i64,
    pub prev_block_hash: [u8; 32],
    pub seed: Seed,
    pub state_hash: [u8; 32],
    pub generator_bls_pubkey: [u8; 96],
    pub gas_limit: u64,

    // Block hash
    pub hash: [u8; 32],

    // Non-hashable fields
    pub cert: Certificate,
}

impl Header {
    /// Marshal hashable fields.
    ///
    /// Param `fixed_size_seed` changes the way seed is marshaled.
    /// In block hashing, header seed is fixed-size field while in wire
    /// message marshaling it is variable-length field.
    fn marshal_hashable<W: Write>(
        &self,
        w: &mut W,
        fixed_size_seed: bool,
    ) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&(self.timestamp as u64).to_le_bytes())?;
        w.write_all(&self.prev_block_hash[..])?;

        if fixed_size_seed {
            w.write_all(&self.seed.inner()[..])?;
        } else {
            Self::write_var_le_bytes(w, &self.seed.inner()[..])?;
        }

        w.write_all(&self.state_hash[..])?;
        w.write_all(&self.generator_bls_pubkey[..])?;
        w.write_all(&self.gas_limit.to_le_bytes())?;

        Ok(())
    }

    fn unmarshal_hashable<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let version = buf[0];

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let height = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let timestamp = u64::from_le_bytes(buf) as i64;

        let mut prev_block_hash = [0u8; 32];
        r.read_exact(&mut prev_block_hash[..])?;

        let seed = Self::read_var_le_bytes(r)?;

        let mut state_hash = [0u8; 32];
        r.read_exact(&mut state_hash[..])?;

        let mut generator_bls_pubkey = [0u8; 96];
        r.read_exact(&mut generator_bls_pubkey[..])?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let gas_limit = u64::from_le_bytes(buf);

        Ok(Header {
            version,
            height,
            timestamp,
            gas_limit,
            prev_block_hash,
            seed: Seed::new(seed),
            generator_bls_pubkey,
            state_hash,
            hash: [0; 32],
            cert: Default::default(),
        })
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.marshal_hashable(w, false)?;

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

impl Default for Header {
    fn default() -> Self {
        Header {
            version: 0,
            height: 0,
            timestamp: 0,
            gas_limit: 0,
            prev_block_hash: Default::default(),
            seed: Default::default(),
            generator_bls_pubkey: [0; 96],
            state_hash: Default::default(),
            hash: Default::default(),
            cert: Default::default(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Transaction {}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub header: Header,
    pub txs: Vec<Transaction>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "block height: {}", self.header.height)
    }
}

impl Serializable for Block {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;

        let txs_num = self.txs.len() as u8;
        w.write_all(&txs_num.to_le_bytes())?;

        // TODO: write transactions

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = Header::read(r)?;

        // Read txs num
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        Ok(Block {
            header,
            txs: vec![],
        })
    }
}

impl Block {
    pub fn new(header: Header, txs: Vec<Transaction>) -> io::Result<Self> {
        let mut b = Block { header, txs };
        b.calculate_hash()?;
        Ok(b)
    }

    pub fn calculate_hash(&mut self) -> io::Result<()> {
        // Call hasher only if header.hash is empty
        if self.header.hash != Hash::default() {
            return Ok(());
        }

        let mut hasher = sha3::Sha3_256::new();
        self.header.marshal_hashable(&mut hasher, true)?;

        self.header.hash = hasher.finalize().into();

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConsensusError {
    InvalidBlock,
    InvalidSignature,
    InvalidMsgType,
    FutureEvent,
    PastEvent,
    NotCommitteeMember,
    NotImplemented,
    NotReady,
    MaxStepReached,
    ChildTaskTerminated,
    Canceled,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Signature(pub [u8; 48]);
impl Signature {
    pub fn is_zeroed(&self) -> bool {
        self.0 == [0; 48]
    }

    pub fn inner(&self) -> [u8; 48] {
        self.0
    }

    pub fn new(value: [u8; 48]) -> Signature {
        Signature(value)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature([0; 48])
    }
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

pub fn spawn_send_reduction<T: Operations + 'static>(
    candidate: Block,
    pubkey: ConsensusPublicKey,
    ru: RoundUpdate,
    step: u8,
    mut outbound: PendingQueue,
    mut inbound: PendingQueue,
    executor: Arc<Mutex<T>>,
) {
    tokio::spawn(async move {
        if let Err(e) = executor.lock().await.verify_state_transition(
            crate::contract_state::CallParams::default(),
        ) {
            tracing::error!("verify state transition failed with err: {:?}", e);
            return;
        }

        let hdr = messages::Header {
            pubkey_bls: pubkey,
            round: ru.round,
            step,
            block_hash: candidate.header.hash,
            topic: Topics::Reduction as u8,
        };

        let signed_hash = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        // Sign and construct reduction message
        let msg = Message::new_reduction(
            hdr,
            messages::payload::Reduction { signed_hash },
        );

        //   publish
        outbound.send(msg.clone()).await.unwrap_or_else(|err| {
            tracing::error!("unable to publish reduction msg {:?}", err)
        });

        // Register my vote locally
        inbound.send(msg).await.unwrap_or_else(|err| {
            tracing::error!("unable to register reduction msg {:?}", err)
        });
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topics {
    // Consensus main loop topics
    Candidate = 15,
    NewBlock = 16,
    Reduction = 17,

    // Consensus Agreement loop topics
    Agreement = 18,
    AggrAgreement = 19,

    Unknown = 100,
}

impl Default for Topics {
    fn default() -> Self {
        Topics::Unknown
    }
}

impl From<Topics> for u8 {
    fn from(t: Topics) -> Self {
        t as u8
    }
}

impl From<u8> for Topics {
    fn from(v: u8) -> Self {
        if v == Topics::NewBlock as u8 {
            return Topics::NewBlock;
        }

        if v == Topics::Reduction as u8 {
            return Topics::Reduction;
        }

        if v == Topics::Agreement as u8 {
            return Topics::Agreement;
        }

        if v == Topics::AggrAgreement as u8 {
            return Topics::AggrAgreement;
        }

        Topics::Unknown
    }
}

pub trait Database: Send + Sync {
    fn store_candidate_block(&mut self, b: Block);
    fn get_candidate_block_by_hash(&self, h: &Hash) -> Option<(Hash, Block)>;
    fn delete_candidate_blocks(&mut self);
}
