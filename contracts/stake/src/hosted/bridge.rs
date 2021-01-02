// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, Counter};
use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature, APK};
use dusk_plonk::prelude::*;

const PAGE_SIZE: usize = 1024 * 4;

type BS = BridgeStore<Id32>;
type QueryIndex = u16;
type TransactionIndex = u16;

fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(&bytes[..], store.clone());

    // read self.
    let slf: Contract<BS> = Canon::<BS>::read(&mut source)?;

    // read query id
    let qid: QueryIndex = Canon::<BS>::read(&mut source)?;
    match qid {
        ops::FIND_STAKE => {
            // Read counter
            let w_i: Counter = Canon::<BS>::read(&mut source)?;
            // Read pk
            let pk: APK = Canon::<BS>::read(&mut source)?;
            // Get the first stake we can find
            let ret = slf.find_stake(w_i, pk)?;
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            Canon::<BS>::write(&ret, &mut sink)
        }
        _ => panic!(""),
    }
}

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    let _ = query(bytes);
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(bytes, store.clone());

    // read self.
    let mut slf: Contract<BS> = Canon::<BS>::read(&mut source)?;
    // read transaction id
    let qid: TransactionIndex = Canon::<BS>::read(&mut source)?;
    match qid {
        ops::STAKE => {
            // Read host-sent args
            let block_height: u64 = Canon::<BS>::read(&mut source)?;
            let value: u64 = Canon::<BS>::read(&mut source)?;
            let public_key: APK = Canon::<BS>::read(&mut source)?;
            // let spending_proof: Proof = Canon::<BS>::read(&mut source)?;
            // let public_inp_len: u8 = Canon::<BS>::read(&mut source)?;
            // let public_inp_bytes: [[u8; 33]; 1] =
            //     Canon::<BS>::read(&mut source)?;
            // Call stake contract fn
            let (w_i, res) = slf.stake(
                block_height,
                value,
                public_key,
                /* spending_proof,
                 * public_inp_len,
                 * public_inp_bytes, */
            );
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&(w_i, res), &mut sink)
        }
        ops::EXTEND_STAKE => {
            // Read host-sent args
            let w_i: Counter = Canon::<BS>::read(&mut source)?;
            let public_key: APK = Canon::<BS>::read(&mut source)?;
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            let res = slf.extend_stake(w_i, public_key, sig);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&res, &mut sink)
        }
        ops::WITHDRAW_STAKE => {
            // Read host-sent args
            let block_height: u64 = Canon::<BS>::read(&mut source)?;
            let w_i: Counter = Canon::<BS>::read(&mut source)?;
            let public_key: APK = Canon::<BS>::read(&mut source)?;
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            // XXX: note missing
            let res = slf.withdraw_stake(
                block_height,
                w_i,
                public_key,
                sig, /* note */
            );
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // Return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // Return result
            Canon::<BS>::write(&res, &mut sink)
        }
        ops::SLASH => {
            // Read host-sent args
            let public_key: APK = Canon::<BS>::read(&mut source)?;
            let round: u64 = Canon::<BS>::read(&mut source)?;
            let step: u8 = Canon::<BS>::read(&mut source)?;
            let message_1: BlsScalar = Canon::<BS>::read(&mut source)?;
            let message_2: BlsScalar = Canon::<BS>::read(&mut source)?;
            let signature_1: Signature = Canon::<BS>::read(&mut source)?;
            let signature_2: Signature = Canon::<BS>::read(&mut source)?;
            // XXX: note missing
            let res = slf.slash(
                public_key,
                round,
                step,
                message_1,
                message_2,
                signature_1,
                signature_2,
                /* note */
            );
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // Return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // Return result
            Canon::<BS>::write(&res, &mut sink)
        }
        _ => panic!(""),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}
