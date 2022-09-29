// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.
use crate::messages;

use crate::messages::Message;
use crate::util::pending_queue::PendingQueue;
use crate::util::pubkey::PublicKey;
use bytes::{Buf, BufMut, BytesMut};
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::Serializable;

use std::fmt;

#[derive(Default, Debug, Copy, Clone)]
#[allow(unused)]
pub struct RoundUpdate {
    pub round: u64,
    pub seed: [u8; 32],
    pub hash: [u8; 32],
    pub timestamp: i64,
    pub pubkey_bls: PublicKey,
    pub secret_key: SecretKey, // TODO: should be here?? SecretKey
}

impl RoundUpdate {
    pub fn new(round: u64, pubkey_bls: PublicKey, secret_key: SecretKey) -> Self {
        RoundUpdate {
            round,
            pubkey_bls,
            secret_key,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Default, Debug, Clone)]
pub struct Transaction {}

#[derive(Default, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub txs: Vec<Transaction>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "block height: {}", self.header.height)
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
}

#[derive(Debug, Copy, Clone)]
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

pub fn sign(
    sk: dusk_bls12_381_sign::SecretKey,
    pk: dusk_bls12_381_sign::PublicKey,
    hdr: messages::Header,
) -> [u8; 48] {
    let mut msg = BytesMut::with_capacity(hdr.block_hash.len() + 8 + 1);
    msg.put_u64_le(hdr.round);
    msg.put_u8(hdr.step);
    msg.put(&hdr.block_hash[..]);

    sk.sign(&pk, msg.bytes()).to_bytes()
}

pub fn verify_signature(
    hdr: &messages::Header,
    signature: [u8; 48],
) -> Result<(), dusk_bls12_381_sign::Error> {
    let sig = dusk_bls12_381_sign::Signature::from_bytes(&signature)?;

    dusk_bls12_381_sign::APK::from(&hdr.pubkey_bls.to_bls_pk()).verify(
        &sig,
        marshal_signable_vote(hdr.round, hdr.step, hdr.block_hash).bytes(),
    )
}

pub fn marshal_signable_vote(round: u64, step: u8, block_hash: [u8; 32]) -> BytesMut {
    let mut msg = BytesMut::with_capacity(block_hash.len() + 8 + 1);
    msg.put_u64_le(round);
    msg.put_u8(step);
    msg.put(&block_hash[..]);

    msg
}

pub fn spawn_send_reduction(
    candidate: Block,
    pubkey: PublicKey,
    ru: RoundUpdate,
    step: u8,
    mut outbound: PendingQueue,
    mut inbound: PendingQueue,
) {
    tokio::spawn(async move {
        // TODO: VerifyStateTransition call here
        // Simulate VerifyStateTransition execution time
        // tokio::time::sleep(Duration::from_secs(3)).await;

        let hdr = messages::Header {
            pubkey_bls: pubkey,
            round: ru.round,
            step,
            block_hash: candidate.header.hash,
        };

        // Sign and construct reduction message
        let msg = Message::new_reduction(
            hdr,
            messages::payload::Reduction {
                signed_hash: sign(ru.secret_key, ru.pubkey_bls.to_bls_pk(), hdr),
            },
        );

        //   publish
        outbound
            .send(msg.clone())
            .await
            .unwrap_or_else(|err| tracing::error!("unable to publish reduction msg {:?}", err));

        // Register my vote locally
        inbound
            .send(msg)
            .await
            .unwrap_or_else(|err| tracing::error!("unable to register reduction msg {:?}", err));
    });
}
