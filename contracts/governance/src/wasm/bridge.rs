// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical::{Canon, Sink, Source};
use dusk_abi::{ContractState, ReturnValue};

const PAGE_SIZE: usize = 1024 * 32;

#[no_mangle]
fn q(_bytes: &mut [u8; PAGE_SIZE]) {}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let mut contract =
        GovernanceContract::decode(&mut source).expect("Failed to read state");
    let tid = u8::decode(&mut source).expect("Failed to read tx ID");

    match tid {
        TX_PAUSE => {
            let (seed, signature) = Canon::decode(&mut source)
                .expect("[TX_PAUSE] Arguments parsing failed");

            contract.pause(seed, signature).unwrap();
        }

        TX_UNPAUSE => {
            let (seed, signature) = Canon::decode(&mut source)
                .expect("[TX_UNPAUSE] Arguments parsing failed");

            contract.unpause(seed, signature).unwrap();
        }

        TX_ALLOW => {
            let (seed, signature, address) = Canon::decode(&mut source)
                .expect("[TX_ALLOW] Arguments parsing failed");

            contract.allow(seed, signature, address).unwrap();
        }

        TX_BLOCK => {
            let (seed, signature, address) = Canon::decode(&mut source)
                .expect("[TX_BLOCK] Arguments parsing failed");

            contract.block(seed, signature, address).unwrap();
        }

        TX_MINT => {
            let (seed, signature, address, value) = Canon::decode(&mut source)
                .expect("[TX_MINT] Arguments parsing failed");

            contract.mint(seed, signature, address, value).unwrap();
        }

        TX_BURN => {
            let (seed, signature, address, value) = Canon::decode(&mut source)
                .expect("[TX_BURN] Arguments parsing failed");

            contract.burn(seed, signature, address, value).unwrap();
        }

        TX_TRANSFER => {
            let (seed, signature, batch) = Canon::decode(&mut source)
                .expect("[TX_TRANSFER] Arguments parsing failed");

            contract.transfer(seed, signature, batch).unwrap();
        }

        _ => panic!("Tx id not implemented"),
    }

    let mut sink = Sink::new(&mut bytes[..]);

    ContractState::from_canon(&contract).encode(&mut sink);
    ReturnValue::from_canon(&true).encode(&mut sink);
}
