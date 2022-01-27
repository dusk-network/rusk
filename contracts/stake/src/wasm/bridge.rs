// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical::{Canon, Sink, Source};
use dusk_abi::{ContractState, ReturnValue};
use dusk_bls12_381_sign::{PublicKey, Signature};
use phoenix_core::Note;
use rusk_abi::PaymentInfo;

use alloc::vec::Vec;

const PAGE_SIZE: usize = 1024 * 32;

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let _ = StakeContract::decode(&mut source).expect("Failed to read state");
    let qid = u8::decode(&mut source).expect("Failed to read query ID");

    let ret = match qid {
        rusk_abi::PAYMENT_INFO => {
            ReturnValue::from_canon(&PaymentInfo::Transparent(None))
        }

        _ => panic!("Undefined query ID"),
    };

    let mut sink = Sink::new(&mut bytes[..]);

    ret.encode(&mut sink);
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    let mut source = Source::new(bytes);

    let mut contract =
        StakeContract::decode(&mut source).expect("Failed to read state");
    let tid = u8::decode(&mut source).expect("Failed to read tx ID");

    match tid {
        TX_STAKE => {
            let (pk, signature, value, spend_proof): (
                PublicKey,
                Signature,
                u64,
                Vec<u8>,
            ) = Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.stake(pk, signature, value, spend_proof);
        }

        TX_EXTEND => {
            let (pk, signature): (PublicKey, Signature) =
                Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.extend(pk, signature);
        }

        TX_WITHDRAW => {
            let (pk, signature, note, withdraw_proof): (
                PublicKey,
                Signature,
                Note,
                Vec<u8>,
            ) = Canon::decode(&mut source).expect("Failed to parse arguments");

            contract.withdraw(pk, signature, note, withdraw_proof);
        }

        _ => panic!("Tx id not implemented"),
    }

    let mut sink = Sink::new(&mut bytes[..]);

    ContractState::from_canon(&contract).encode(&mut sink);
    ReturnValue::from_canon(&true).encode(&mut sink);
}
