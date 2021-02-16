// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, TransferContract};

use canonical::{
    BridgeStore as BridgeStoreCanon, ByteSink, ByteSource, Canon, Id32, Store,
};
use dusk_abi::{ContractState, ReturnValue};

const PAGE_SIZE: usize = 1024 * 8;

type BridgeStore = BridgeStoreCanon<Id32>;

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    if let Err(_) = query(bytes) {
        // TODO handle error
        // https://github.com/dusk-network/rusk/issues/193
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    if let Err(_) = transaction(bytes) {
        // TODO handle error
        // https://github.com/dusk-network/rusk/issues/193
    }
}

fn query(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BridgeStore as Store>::Error> {
    let bridge = BridgeStore::default();
    let mut source = ByteSource::new(&bytes[..], &bridge);
    let contract: TransferContract<BridgeStore> = Canon::read(&mut source)?;

    let query = Canon::read(&mut source)?;
    let ret = match query {
        ops::QR_BALANCE => {
            let address = Canon::read(&mut source)?;

            let ret = contract.balance(address);
            ReturnValue::from_canon(&ret, &bridge)?
        }

        ops::QR_ROOT => {
            let ret = contract.root();
            ReturnValue::from_canon(&ret, &bridge)?
        }

        ops::QR_NOTES_FROM_HEIGHT => {
            let block_height = Canon::read(&mut source)?;

            let ret = contract.notes_from_height(block_height);
            ReturnValue::from_canon(&ret, &bridge)?
        }

        ops::QR_OPENING => {
            let pos = Canon::read(&mut source)?;

            let ret = contract.opening(pos);
            ReturnValue::from_canon(&ret, &bridge)?
        }

        // TODO Define error strategy
        // https://github.com/dusk-network/rusk/issues/193
        _ => ReturnValue::from_canon(&(), &bridge)?,
    };

    let mut sink = ByteSink::new(&mut bytes[..], &bridge);
    Canon::write(&ret, &mut sink)?;

    Ok(())
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BridgeStore as Store>::Error> {
    let bridge = BridgeStore::default();
    let mut source = ByteSource::new(&bytes[..], &bridge);
    let mut contract: TransferContract<BridgeStore> = Canon::read(&mut source)?;

    let call = Canon::read(&mut source)?;
    let ret = contract.execute(call);

    let mut sink = ByteSink::new(&mut bytes[..], &bridge);

    let state = ContractState::from_canon(&contract, &bridge)?;
    let ret = ReturnValue::from_canon(&ret, &bridge)?;

    Canon::write(&state, &mut sink)?;
    Canon::write(&ret, &mut sink)?;

    Ok(())
}
