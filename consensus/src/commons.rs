// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.
use crate::messages;
use crate::util::pubkey::PublicKey;
use bytes::{Buf, BufMut, BytesMut};
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::Serializable;
use std::fmt;

#[derive(Default, Debug, Copy, Clone)]
#[allow(unused)]
pub struct RoundUpdate {
    pub(crate) round: u64,
    pub(crate) seed: [u8; 32],
    pub(crate) hash: [u8; 32],
    pub(crate) pubkey_bls: PublicKey,
    pub(crate) secret_key: SecretKey, // TODO: should be here?? SecretKey
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

#[derive(Default, Debug, Clone)]
pub struct Header {
    pub version: u8,
    pub height: u64,
    pub timestamp: i64,
    pub gas_limit: u64,
    pub prev_block_hash: [u8; 32],
    pub seed: [u8; 32],
    pub generator_bls_pubkey: [u8; 32], // TODO: size should be 96
    pub state_hash: [u8; 32],
    pub hash: [u8; 32],
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

#[derive(Debug)]
pub enum SelectError {
    Continue,
    Canceled,
    Timeout,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConsensusError {
    // TODO: Rename InvalidRoundStep
    InvalidRoundStep,
    InvalidBlock,
    InvalidSignature,
    InvalidMsgType,
    FutureEvent,
    NotCommitteeMember,
    NotImplemented,
    NotReady,
    MaxStepReached,
}

// TODO: This to be replaced with bls::Signature

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

    sk.sign(&pk, msg.bytes()).to_bytes().into()
}

pub fn verify_signature(
    hdr: &messages::Header,
    signature: [u8; 48],
) -> Result<(), dusk_bls12_381_sign::Error> {
    let sig = dusk_bls12_381_sign::Signature::from_bytes(&signature.into())?;

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

// TODO: Encapsulate all run params in a single struct as they are used in another 9 functions/calls as input

/*
pub struct PhaseContext<'a> {
    cancel_recv: &'a mut oneshot::Receiver<Context>,

    inbound_msgs: &'a mut mpsc::Receiver<Message>,
    future_msgs: &'a mut Queue<Message>,
    outbound_msgs: &'a mut mpsc::Sender<Message>,

    committee: Committee,

    ru: RoundUpdate,
    step: u8,
}
*/
