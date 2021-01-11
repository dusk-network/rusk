// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(test)]
#![cfg(feature = "host")]

use canonical_host::{MemStore, Remote, Wasm};
use dusk_plonk::proof_system::proof::Proof;
use external::RuskExternals;
use phoenix_core::Note;
use transfer_contract::Contract;

mod external;

const BYTECODE: &'static [u8] = include_bytes!(
    "../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

#[test]
fn transfer_contract() {
    let store = MemStore::new();
    let contract = Contract::default();
    let wasm = Wasm::new(contract, BYTECODE);
    let mut remote = Remote::new(wasm, &store).unwrap();

    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();

    let note: Note = unsafe { std::mem::zeroed() };
    let proof: Proof = unsafe { std::mem::zeroed() };
    /*
    let mut pub_inputs = [[0u8; 33]; 1];
    */
    let tx = Contract::<MemStore>::send_to_contract_transparent(
        note, proof,
        //pub_inputs,
    );

    let response = cast
        .transact(&tx, store.clone(), RuskExternals::default())
        .unwrap();

    cast.commit().unwrap();

    assert!(response);
}
