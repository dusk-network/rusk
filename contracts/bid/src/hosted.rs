// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Hosted interface for the Bid Contract.
//!
//! Here the interface of the contract that will run inside the hosted
//! environment (WASM instance) is defined and implemented.

mod transaction;

use crate::{ops, Contract};
use alloc::vec::Vec;
use canonical::{Canon, CanonError, Sink, Source};
use dusk_abi::{ContractState, ReturnValue};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::{PublicKey, StealthAddress};
use dusk_schnorr::Signature;
use phoenix_core::{Message, Note};
use rusk_abi::PAYMENT_INFO;

const PAGE_SIZE: usize = 1024 * 4;

type TransactionIndex = u8;
type QueryIndex = u8;

fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    let mut source = Source::new(&bytes[..]);

    // read self.
    let slf = Contract::decode(&mut source)?;

    // read query id
    let qid = QueryIndex::decode(&mut source)?;
    match qid {
        // read_value (&Self) -> i32
        PAYMENT_INFO => {
            let mut sink = Sink::new(&mut bytes[..]);

            ReturnValue::from_canon(&super::contract_constants::PAYMENT_INFO)
                .encode(&mut sink);
            Ok(())
        }
        _ => panic!("Unimplemented OP"),
    }
}

fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    let mut source = Source::new(bytes);

    // read self.
    let mut slf = Contract::decode(&mut source)?;
    // read transaction id
    let qid = TransactionIndex::decode(&mut source)?;
    match qid {
        ops::BID => {
            // Read host-sent args
            let message = Message::decode(&mut source)?;
            let hashed_secret = BlsScalar::decode(&mut source)?;
            let stealth_addr = StealthAddress::decode(&mut source)?;
            // Fat pointer to the Proof objects.
            let correctness_proof = Vec::<u8>::decode(&mut source)?;
            let spending_proof = Vec::<u8>::decode(&mut source)?;
            // Call bid contract fn
            slf.bid(
                message,
                hashed_secret,
                stealth_addr,
                correctness_proof,
                spending_proof,
            );
            let mut sink = Sink::new(&mut bytes[..]);

            // return new state
            ContractState::from_canon(&slf).encode(&mut sink);
            // return result
            ReturnValue::from_canon(&true).encode(&mut sink);
            Ok(())
        }
        ops::WITHDRAW => {
            // Read host-sent args
            let sig = Signature::decode(&mut source)?;
            let pk = PublicKey::decode(&mut source)?;
            let note = Note::decode(&mut source)?;
            let spending_proof = Vec::<u8>::decode(&mut source)?;
            slf.withdraw(sig, pk, note, spending_proof);
            let mut sink = Sink::new(&mut bytes[..]);

            // return new state
            ContractState::from_canon(&slf).encode(&mut sink);
            // return result
            ReturnValue::from_canon(&true).encode(&mut sink);
            Ok(())
        }
        ops::EXTEND_BID => {
            // Read host-sent args
            let sig = Signature::decode(&mut source)?;
            let pk = PublicKey::decode(&mut source)?;
            slf.extend_bid(sig, pk);
            let mut sink = Sink::new(&mut bytes[..]);

            // return new state
            ContractState::from_canon(&slf).encode(&mut sink);
            // return result
            ReturnValue::from_canon(&true).encode(&mut sink);
            Ok(())
        }
        _ => panic!("Unimplemented OP"),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    query(bytes).unwrap()
}
