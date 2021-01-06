// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, PublicKeys};
use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
use dusk_bls12_381_sign::{Signature, APK};

const PAGE_SIZE: usize = 1024 * 16;

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
        ops::GET_BALANCE => {
            // Read pk
            let pk: APK = Canon::<BS>::read(&mut source)?;
            // Get the first stake we can find
            let ret = slf.get_balance(pk)?;
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            Canon::<BS>::write(&ret, &mut sink)
        }
        ops::GET_WITHDRAWAL_TIME => {
            // Read pk
            let pk: APK = Canon::<BS>::read(&mut source)?;
            // Get the first stake we can find
            let ret = slf.get_withdrawal_time(pk)?;
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
        ops::DISTRIBUTE => {
            // Read host-sent args
            let value: u64 = Canon::<BS>::read(&mut source)?;
            let public_keys: PublicKeys<BS> = Canon::<BS>::read(&mut source)?;
            // Call stake contract fn
            let res = slf.distribute(value, public_keys);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&res, &mut sink)
        }
        ops::WITHDRAW => {
            // Read host-sent args
            let block_height: u64 = Canon::<BS>::read(&mut source)?;
            let public_key: APK = Canon::<BS>::read(&mut source)?;
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            // TODO: decode note
            let res = slf.withdraw(block_height, public_key, sig);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&res, &mut sink)
        }
        _ => panic!(""),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}
