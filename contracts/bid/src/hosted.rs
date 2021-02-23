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
use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
use dusk_blindbid::Bid;
use dusk_pki::PublicKey;
use phoenix_core::Note;
use schnorr::Signature;

const PAGE_SIZE: usize = 1024 * 4;

type BS = BridgeStore<Id32>;
type TransactionIndex = u8;

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(bytes, &store);

    // read self.
    let mut slf: Contract<BS> = Canon::<BS>::read(&mut source)?;
    // read transaction id
    let qid: TransactionIndex = Canon::<BS>::read(&mut source)?;
    match qid {
        ops::BID => {
            // Read host-sent args
            let bid: Bid = Canon::<BS>::read(&mut source)?;
            // Fat pointer to the Proof objects.
            let correctness_proof: Vec<u8> = Canon::<BS>::read(&mut source)?;
            let spending_proof: Vec<u8> = Canon::<BS>::read(&mut source)?;
            // Call bid contract fn
            let (err_flag, idx) =
                slf.bid(bid, correctness_proof, spending_proof);
            let mut sink = ByteSink::new(&mut bytes[..], &store);
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&(err_flag, (idx as u64)), &mut sink)
        }
        ops::WITHDRAW => {
            // Read host-sent args
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            let pk: PublicKey = Canon::<BS>::read(&mut source)?;
            let note: Note = Canon::<BS>::read(&mut source)?;
            let spending_proof: Vec<u8> = Canon::<BS>::read(&mut source)?;
            let block_height: u64 = Canon::<BS>::read(&mut source)?;
            let exec_res =
                slf.withdraw(sig, pk, note, spending_proof, block_height);
            let mut sink = ByteSink::new(&mut bytes[..], &store);
            // Return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // Return result
            Canon::<BS>::write(&exec_res, &mut sink)
        }
        ops::EXTEND_BID => {
            // Read host-sent args
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            let pk: PublicKey = Canon::<BS>::read(&mut source)?;
            let exec_res = slf.extend_bid(sig, pk);
            let mut sink = ByteSink::new(&mut bytes[..], &store);
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&exec_res, &mut sink)
        }
        _ => panic!("Unimplemented OP"),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}
