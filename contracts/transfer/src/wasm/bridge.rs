// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Call, Transfer};

use alloc::vec::Vec;
use canonical::{
    BridgeStore as BridgeStoreCanon, ByteSink, ByteSource, Canon, Id32, Store,
};
use dusk_abi::{ContractState, ReturnValue};
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Fee, Note};

const PAGE_SIZE: usize = 1024 * 4;

type BridgeStore = BridgeStoreCanon<Id32>;

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    if let Err(_) = query(bytes) {
        // TODO handle error
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    if let Err(_) = transaction(bytes) {
        // TODO handle error
    }
}

fn query(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BridgeStore as Store>::Error> {
    let bridge = BridgeStore::default();
    let mut source = ByteSource::new(&bytes[..], &bridge);
    let contract: Transfer<BridgeStore> = Canon::read(&mut source)?;

    let query = Canon::read(&mut source)?;
    match query {
        _ => (), // TODO report unexpected ID
    }

    // TODO Implement spurious functions
    let _ = contract;

    Ok(())
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BridgeStore as Store>::Error> {
    let bridge = BridgeStore::default();
    let mut source = ByteSource::new(&bytes[..], &bridge);
    let mut contract: Transfer<BridgeStore> = Canon::read(&mut source)?;

    let tx = Canon::read(&mut source)?;
    let ret = match tx {
        ops::TX_EXECUTE => {
            let anchor: BlsScalar = Canon::read(&mut source)?;
            let nullifiers: Vec<BlsScalar> = Canon::read(&mut source)?;
            let crossover: Crossover = Canon::read(&mut source)?;
            let notes: Vec<Note> = Canon::read(&mut source)?;
            let fee: Fee = Canon::read(&mut source)?;
            let spend_proof: Vec<u8> = Canon::read(&mut source)?;
            let call: Call = Canon::read(&mut source)?;

            contract.execute(
                anchor,
                nullifiers,
                crossover,
                notes,
                fee,
                spend_proof,
                call,
            )
        }

        _ => false, // TODO report unexpected ID
    };

    let mut sink = ByteSink::new(&mut bytes[..], &bridge);

    let state = ContractState::from_canon(&contract, &bridge)?;
    let ret = ReturnValue::from_canon(&ret, &bridge)?;

    Canon::write(&state, &mut sink)?;
    Canon::write(&ret, &mut sink)?;

    Ok(())
}
