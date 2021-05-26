// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Hosted interface for the Bid Contract.
//!
//! Here the interface of the contract that will run inside the hosted
//! environment (WASM instance) is defined and implemented.

mod transaction;

use crate::{ops, Contract};
use alloc::vec::Vec;
use canonical::{Canon, CanonError, Sink, Source};
use dusk_blindbid::Bid;
use dusk_pki::PublicKey;
use dusk_schnorr::Signature;
use phoenix_core::Note;

const PAGE_SIZE: usize = 1024 * 4;

type TransactionIndex = u8;

fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    let mut source = Source::new(bytes);

    // read self.
    let mut slf = Contract::decode(&mut source)?;
    // read transaction id
    let qid = TransactionIndex::decode(&mut source)?;
    match qid {
        ops::BID => {
            // Read host-sent args
            let bid = Bid::decode(&mut source)?;
            // Fat pointer to the Proof objects.
            let correctness_proof = Vec::<u8>::decode(&mut source)?;
            let spending_proof = Vec::<u8>::decode(&mut source)?;
            // Call bid contract fn
            let success = slf.bid(bid, correctness_proof, spending_proof);
            let mut sink = Sink::new(&mut bytes[..]);
            // return new state
            slf.encode(&mut sink);
            // return result
            Ok(success.encode(&mut sink))
        }
        ops::WITHDRAW => {
            // Read host-sent args
            let sig = Signature::decode(&mut source)?;
            let pk = PublicKey::decode(&mut source)?;
            let note = Note::decode(&mut source)?;
            let spending_proof = Vec::<u8>::decode(&mut source)?;
            let block_height = u64::decode(&mut source)?;
            let exec_res =
                slf.withdraw(sig, pk, note, spending_proof, block_height);
            let mut sink = Sink::new(&mut bytes[..]);
            // Return new state
            slf.encode(&mut sink);
            // Return result
            Ok(exec_res.encode(&mut sink))
        }
        ops::EXTEND_BID => {
            // Read host-sent args
            let sig = Signature::decode(&mut source)?;
            let pk = PublicKey::decode(&mut source)?;
            let exec_res = slf.extend_bid(sig, pk);
            let mut sink = Sink::new(&mut bytes[..]);
            // return new state
            slf.encode(&mut sink);
            // return result
            Ok(exec_res.encode(&mut sink))
        }
        _ => panic!("Unimplemented OP"),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}
