// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, Leaf};

use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
use dusk_plonk::proof_system::proof::Proof;
use phoenix_core::Note;

const PAGE_SIZE: usize = 1024 * 4;

type BS = BridgeStore<Id32>;

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    let _ = query(bytes);
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}

fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(&bytes[..], store.clone());

    let contract: Contract<BS> = Canon::<BS>::read(&mut source)?;
    let qid: u8 = Canon::<BS>::read(&mut source)?;

    match qid {
        _ => Ok(()),
    }
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(bytes, store.clone());

    let mut contract: Contract<BS> = Canon::<BS>::read(&mut source)?;
    let qid: u8 = Canon::<BS>::read(&mut source)?;

    match qid {
        ops::TX_SEND_TO_CONTRACT_TRANSPARENT => {
            let note: Note = Canon::<BS>::read(&mut source)?;
            let spending_proof: Proof = Canon::<BS>::read(&mut source)?;
            /*
            let pub_inputs: [[u8; 33]; 1] = Canon::<BS>::read(&mut source)?;
            */

            let ret = contract.send_to_contract_transparent(
                note,
                spending_proof,
                /*
                pub_inputs,
                */
            );

            let mut sink = ByteSink::new(&mut bytes[..], store);

            Canon::<BS>::write(&contract, &mut sink)?;
            Canon::<BS>::write(&ret, &mut sink)?;

            Ok(())
        }

        _ => Ok(()),
    }
}
