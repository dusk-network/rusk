// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

#![no_std]
use cake_rusk as cake;

#[cake::contract(version = "0.0.1")]
mod transfer {
    use phoenix_abi::{Input, Note, Proof, PublicKey};

    type Inputs = [Input; Input::MAX];
    type Notes = [Note; Note::MAX];

    pub fn transfer(inputs: Inputs, notes: Notes, proof: Proof) -> i32 {
        if !phoenix_abi::verify(&inputs, &notes, &proof) {
            return 0;
        }

        phoenix_abi::store(&inputs, &notes, &proof) as i32
    }

    pub fn approve(
        inputs: Inputs,
        notes: Notes,
        pk: PublicKey,
        value: u64,
        proof: Proof,
    ) -> i32 {
        if !phoenix_abi::verify(&inputs, &notes, &proof) {
            return 0;
        }
        phoenix_abi::store(&inputs, &notes, &proof);
        let current_value = dusk_abi::get_storage(&pk).unwrap_or(0);
        dusk_abi::set_storage(&pk, value + current_value);
        1
    }

    pub fn transfer_from(
        sender: PublicKey,
        recipient: PublicKey,
        value: u64,
    ) -> i32 {
        let approved_value = dusk_abi::get_storage(&sender).unwrap_or(0);
        if value > approved_value {
            return 0;
        }
        dusk_abi::set_storage(&sender, approved_value - value);
        phoenix_abi::credit(value, &recipient);
        1
    }
}
