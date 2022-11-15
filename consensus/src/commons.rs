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
use std::io::{self, Read, Write};

use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default, Debug)]
#[allow(unused)]
pub struct RoundUpdate {
    pub round: u64,
    pub seed: [u8; 32],
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
    ) -> Self {
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub height: u64,
    pub timestamp: i64,
    pub gas_limit: u64,
    pub prev_block_hash: [u8; 32],
    pub seed: [u8; 32],
    pub generator_bls_pubkey: [u8; 96],
    pub state_hash: [u8; 32],
    pub hash: [u8; 32],
}

impl Header {
    fn marshal_hashable<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&(self.timestamp as u64).to_le_bytes())?;
        w.write_all(&self.prev_block_hash[..])?;
        w.write_all(&self.seed[..])?;
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

        let mut seed = [0u8; 32];
        r.read_exact(&mut seed[..])?;

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
            seed,
            generator_bls_pubkey,
            state_hash,
            hash: [0; 32],
        })
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.marshal_hashable(w)?;

        // TODO: marshal certificate

        w.write_all(&self.hash[..])?;

        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut header = Self::unmarshal_hashable(r)?;

        // TODO: read certificate

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

        let txs_num = self.txs.len() as u64;
        w.write_all(&txs_num.to_le_bytes())?;

        // TODO: write transactions

        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = Header::read(r)?;

        // Read txs num
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;

        Ok(Block {
            header,
            txs: vec![],
        })
    }
}

impl Block {
    pub fn new(header: Header, txs: Vec<Transaction>) -> Self {
        let mut b = Block { header, txs };
        b.calculate_hash();
        b
    }

    fn calculate_hash(&mut self) {
        use sha3::Digest;
        let hdr = self.header.clone();

        let mut hasher = sha3::Sha3_256::new();
        hasher.update(hdr.version.to_le_bytes());
        hasher.update(hdr.height.to_le_bytes());
        hasher.update(hdr.timestamp.to_le_bytes());
        hasher.update(hdr.prev_block_hash);
        hasher.update(hdr.seed);
        hasher.update(hdr.state_hash);
        hasher.update(hdr.generator_bls_pubkey);
        hasher.update(hdr.gas_limit.to_le_bytes());

        self.header.hash = hasher.finalize().into();
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

#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 48]);
impl Signature {
    pub fn is_zeroed(&self) -> bool {
        self.0 == [0; 48]
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature([0; 48])
    }
}

// TODO: Apply Hash type instead of u8; 32
pub type Hash = [u8; 32];

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
