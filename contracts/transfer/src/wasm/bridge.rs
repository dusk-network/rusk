// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Call, TransferContract};

use canonical::{
    BridgeStore as BridgeStoreCanon, ByteSink, ByteSource, Canon, Id32, Store,
};
use dusk_abi::{ContractState, ReturnValue};

const PAGE_SIZE: usize = 1024 * 32;

type BridgeStore = BridgeStoreCanon<Id32>;

#[no_mangle]
fn q(_bytes: &mut [u8; PAGE_SIZE]) {
    panic!("Spurious functions are supposed to be called from the state of the contract!");
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).expect("Failed to execute the provided transaction!");
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BridgeStore as Store>::Error> {
    let bridge = BridgeStore::default();
    let mut source = ByteSource::new(&bytes[..], &bridge);
    let mut contract: TransferContract<BridgeStore> = Canon::read(&mut source)?;

    let call: Call = Canon::read(&mut source)?;
    let ret = call.transact(&mut contract);
    let mut sink = ByteSink::new(&mut bytes[..], &bridge);

    // FIXME Clarify if the state should be sent to the bridge if the execution
    // fails https://github.com/dusk-network/rusk/issues/204
    let state = ContractState::from_canon(&contract, &bridge)?;
    let ret = ReturnValue::from_canon(&ret, &bridge)?;

    Canon::write(&state, &mut sink)?;
    Canon::write(&ret, &mut sink)?;

    Ok(())
}
