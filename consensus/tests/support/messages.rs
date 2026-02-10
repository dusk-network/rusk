// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::{Block, Header};
use node_data::message::{Message, Payload, SignedStepMessage};

use dusk_consensus::commons::RoundUpdate;
use dusk_consensus::merkle::merkle_root;

pub fn corrupt_message_signature(msg: &Message) -> Option<Message> {
    let mut corrupted = msg.clone();
    match &mut corrupted.payload {
        Payload::Candidate(c) => {
            let mut sig = *c.candidate.header().signature.inner();
            sig[0] ^= 0x01;
            c.candidate.set_signature(sig.into());
            Some(corrupted)
        }
        Payload::Validation(v) => {
            let mut sig = *v.sign_info.signature.inner();
            sig[0] ^= 0x01;
            v.sign_info.signature = sig.into();
            Some(corrupted)
        }
        Payload::Ratification(r) => {
            let mut sig = *r.sign_info.signature.inner();
            sig[0] ^= 0x01;
            r.sign_info.signature = sig.into();
            Some(corrupted)
        }
        _ => None,
    }
}

pub fn build_candidate_message(ru: &RoundUpdate, iteration: u8) -> Message {
    let mut header = Header::default();
    header.height = ru.round;
    header.iteration = iteration;
    header.prev_block_hash = ru.hash();
    header.generator_bls_pubkey = *ru.pubkey_bls.bytes();
    header.txroot = merkle_root::<[u8; 32]>(&[]);
    header.faultroot = merkle_root::<[u8; 32]>(&[]);

    let block = Block::new(header, vec![], vec![]).expect("valid block");
    let mut candidate = node_data::message::payload::Candidate { candidate: block };
    candidate.sign(&ru.secret_key, ru.pubkey_bls.inner());
    candidate.into()
}
