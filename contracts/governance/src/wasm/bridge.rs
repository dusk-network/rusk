// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical::{Canon, Sink, Source};
use dusk_abi::{ContractState, ReturnValue};
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::Signature;

const PAGE_SIZE: usize = 1024 * 32;

#[no_mangle]
fn q(_bytes: &mut [u8; PAGE_SIZE]) {}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let mut contract = GovernanceContract::decode(&mut source)
        .expect("transact's data should have a GovernanceContract state");

    let signature = Signature::decode(&mut source)
        .expect("transact's data should have a Signature");

    let len_u32 = u32::decode(&mut source)
        .expect("transact's data should have the payload's length");

    let offset = contract.encoded_len() + signature.encoded_len();
    let len = offset + len_u32 as usize;

    let seed = BlsScalar::decode(&mut source)
        .expect("transact's data should have a seed");

    contract
        .verify(seed, signature, &bytes[offset..len])
        .expect("contract's authority should match the signed transact's data");

    let tid = u8::decode(&mut source)
        .expect("transact's data should have a method's ID");

    match tid {
        TX_PAUSE => contract.pause(),
        TX_UNPAUSE => contract.unpause(),
        TX_MINT => {
            let (address, value) = Canon::decode(&mut source)
                .expect("[TX_MINT] arguments should be decoded");

            contract.mint(address, value).unwrap();
        }

        TX_BURN => {
            let (address, value) = Canon::decode(&mut source)
                .expect("[TX_BURN] arguments should be decoded");

            contract.burn(address, value).unwrap();
        }

        TX_TRANSFER => {
            let batch = Canon::decode(&mut source)
                .expect("[TX_TRANSFER] arguments should be decoded");

            contract.transfer(batch).unwrap();
        }

        _ => panic!("Tx id not implemented"),
    }

    let mut sink = Sink::new(&mut bytes[..]);

    ContractState::from_canon(&contract).encode(&mut sink);
    ReturnValue::from_canon(&true).encode(&mut sink);
}
