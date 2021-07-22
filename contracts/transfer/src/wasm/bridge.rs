// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Call, TransferContract};

use canonical::{Canon, CanonError, Sink, Source};
use dusk_abi::{ContractState, ReturnValue};

const PAGE_SIZE: usize = 1024 * 32;

#[no_mangle]
fn q(_bytes: &mut [u8; PAGE_SIZE]) {
    panic!("Spurious functions are supposed to be called from the state of the contract!");
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).expect("Failed to execute the provided transaction!");
}

fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    let mut source = Source::new(bytes);
    let mut contract = TransferContract::decode(&mut source)?;

    dusk_abi::debug!("consumed {}", dusk_abi::gas_consumed());

    dusk_abi::debug!(
        "state struct size {}",
        core::mem::size_of::<TransferContract>()
    );
    dusk_abi::debug!("encoded size {}", contract.encoded_len());

    let call = Call::decode(&mut source)?;
    let ret = call.transact(&mut contract);
    let mut sink = Sink::new(&mut bytes[..]);

    ContractState::from_canon(&contract).encode(&mut sink);
    ReturnValue::from_canon(&ret).encode(&mut sink);

    Ok(())
}
