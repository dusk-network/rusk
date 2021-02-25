// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    ops,
    stake::{Call, StakeContract},
};

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
    let contract: StakeContract<BridgeStore> = Canon::read(&mut source)?;

    let query = Canon::read(&mut source)?;
    let ret = match query {
        ops::QR_FIND_STAKE => {
            let w_i = Canon::read(&mut source)?;
            let pk = Canon::read(&mut source)?;

            let ret = contract.find_stake(w_i, pk)?;
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
    let mut contract: StakeContract<BridgeStore> = Canon::read(&mut source)?;

    let call = Canon::read(&mut source)?;
    let ret = match call {
        Call::Stake {
            value,
            public_key,
            spending_proof,
        } => {
            let ret = contract.stake(value, public_key, spending_proof);
            ReturnValue::from_canon(&ret, &bridge)?
        }
        Call::ExtendStake {
            w_i,
            public_key,
            sig,
        } => {
            let ret = contract.extend_stake(w_i, public_key, sig);
            ReturnValue::from_canon(&ret, &bridge)?
        }
        Call::WithdrawStake {
            w_i,
            public_key,
            sig,
            note,
        } => {
            let ret = contract.withdraw_stake(w_i, public_key, sig, note);
            ReturnValue::from_canon(&ret, &bridge)?
        }
        Call::Slash {
            public_key,
            round,
            step,
            message_1,
            message_2,
            signature_1,
            signature_2,
            note,
        } => {
            let ret = contract.slash(
                public_key,
                round,
                step,
                message_1,
                message_2,
                signature_1,
                signature_2,
                note,
            );
            ReturnValue::from_canon(&ret, &bridge)?
        }
        // TODO Define error strategy
        // https://github.com/dusk-network/rusk/issues/193
        _ => ReturnValue::from_canon(&(), &bridge)?,
    };

    let mut sink = ByteSink::new(&mut bytes[..], &bridge);

    let state = ContractState::from_canon(&contract, &bridge)?;
    let ret = ReturnValue::from_canon(&ret, &bridge)?;

    Canon::write(&state, &mut sink)?;
    Canon::write(&ret, &mut sink)?;

    Ok(())
}
